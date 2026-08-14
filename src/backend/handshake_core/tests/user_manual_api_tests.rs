//! WP-KERNEL-009 UserManual route-level proof against REAL PostgreSQL over a
//! loopback listener (quiet; no foreground window):
//! * MT-201 UserManualBackendApi — list / read / search / link routes.
//! * MT-199 UserManualModelQuickstartBundles — per-area bundles.
//! * MT-200 UserManualInAppAccess — access points resolve against live rows.
//! * MT-203 LegacyManualShimTests — bridge route + compatibility receipt.
//! * MT-204 UserManualFreshnessCheck — current / tampered verdicts.
//! * MT-205 UserManualVisualDebugProof — HTML projection selectors +
//!   navigation reachability.
//! * Resync permission gate (cloud_model / unauthenticated write-deny).
//! * THE doc-vs-runtime keystone: every registry surface is probed against
//!   the REAL full product router (`api::routes`) — a documented route the
//!   router does not serve fails the suite.

mod knowledge_pg_support;
#[allow(dead_code)]
mod user_manual_support;

use handshake_core::api;
use handshake_core::user_manual::fixtures::{
    restore_page_content_hash, tamper_page_content_hash, unreachable_pages,
};
use handshake_core::user_manual::registry::{probe_path, wp009_surface_registry, SurfaceGroup};
use handshake_core::user_manual::seed::{ensure_seeded, QUICKSTART_AREAS};
use handshake_core::user_manual::USER_MANUAL_VERSION;
use knowledge_pg_support::KnowledgePg;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Connection;
use std::{collections::BTreeSet, path::PathBuf};
use user_manual_support::{app_state_for, start_server};

struct ApiFixture {
    kpg: KnowledgePg,
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
}

fn assert_internal_diagnostics_and_palmistry_wired(body: &str) {
    assert!(
        body.contains("internal_diagnostics is WIRED through the native producer and Problems projection")
            || body.contains("internal_diagnostics` is WIRED through the native producer and Problems projection"),
        "UserManual must report internal_diagnostics as WIRED through the native producer and Problems projection"
    );
    assert!(
        body.contains("Palmistry is WIRED through the authenticated watcher and survivor recovery importer"),
        "UserManual must report Palmistry as WIRED through the authenticated watcher and survivor recovery importer"
    );
    assert!(
        !body.contains("internal_diagnostics is DEFERRED-with-reason")
            && !body.contains("internal_diagnostics` is DEFERRED-with-reason")
            && !body.contains("until the separate watcher worktree is merged")
            && !body.contains("from follow-up worktrees"),
        "UserManual must not regress to stale diagnostics worktree deferrals"
    );
}

async fn fixture() -> Option<ApiFixture> {
    let kpg = knowledge_pg_support::knowledge_pg().await?;
    ensure_seeded(&kpg.db).await.expect("seed corpus");
    let state = app_state_for(&kpg.schema_url).await;
    let (base, server) = start_server(api::user_manual::routes(state)).await;
    Some(ApiFixture {
        kpg,
        base,
        _server: server,
        http: reqwest::Client::new(),
    })
}

async fn receipt_exists(kpg: &KnowledgePg, event_id: &str) -> bool {
    let mut conn = kpg.raw_connection().await;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM kernel_event_ledger \
         WHERE event_id = $1 AND event_type = 'KNOWLEDGE_USER_MANUAL_ENTRY_RECORDED')",
    )
    .bind(event_id)
    .fetch_one(&mut conn)
    .await
    .expect("receipt lookup");
    conn.close().await.ok();
    exists
}

/// The `pub fn routes(...)` body of `api/loom.rs`, from the router constructor
/// to its terminating `.with_state(`.
///
/// MT-022 correctness fix: the scraper below walks `.route(` occurrences and
/// ends each route's method block at the NEXT `.route(`. Without this slice the
/// LAST route's block ran to end-of-file, so every `get(`/`patch(`/... token in
/// the ~4600 lines of handler code AFTER the router was falsely attributed to
/// `/workspaces/:workspace_id/loom/views/definitions/:block_id/results`. That
/// invented GET and PATCH surfaces the router does not serve — registering them
/// would have made `mtdoc_every_registry_surface_exists_on_the_real_router` fail
/// with 405 METHOD_NOT_ALLOWED. Slicing to the router body also stops any future
/// `.route(` inside a handler or `mod tests` from being scraped as mounted.
fn loom_router_body() -> &'static str {
    let source = include_str!("../src/api/loom.rs");
    let start = source
        .find("pub fn routes(")
        .expect("api/loom.rs must declare `pub fn routes(`");
    let body = &source[start..];
    let end = body
        .find(".with_state(")
        .expect("api::loom::routes must terminate with `.with_state(`");
    &body[..end]
}

fn loom_router_surfaces_from_source() -> BTreeSet<(String, String)> {
    let source = loom_router_body();
    let mut surfaces = BTreeSet::new();
    let mut remaining = source;

    while let Some(route_start) = remaining.find(".route(") {
        remaining = &remaining[route_start + ".route(".len()..];
        let Some(path_start) = remaining.find('"') else {
            continue;
        };
        let after_path_start = &remaining[path_start + 1..];
        let Some(path_end) = after_path_start.find('"') else {
            continue;
        };
        let path = &after_path_start[..path_end];
        let after_path = &after_path_start[path_end + 1..];
        let next_route = after_path.find(".route(").unwrap_or(after_path.len());
        let route_block = &after_path[..next_route];

        for (needle, method) in [
            ("get(", "GET"),
            (".get(", "GET"),
            ("post(", "POST"),
            (".post(", "POST"),
            ("put(", "PUT"),
            (".put(", "PUT"),
            ("patch(", "PATCH"),
            (".patch(", "PATCH"),
            ("delete(", "DELETE"),
            (".delete(", "DELETE"),
        ] {
            if route_block.contains(needle) {
                surfaces.insert((method.to_string(), path.to_string()));
            }
        }

        remaining = after_path;
    }

    surfaces
}

/// MT-195 negative guard: a Loom route mounted in `api::loom` must be present
/// in the UserManual registry. This catches the opposite direction from the
/// doc-vs-runtime probe: runtime route -> manual inventory.
#[test]
fn mtdoc_every_loom_router_route_is_in_surface_registry() {
    let registry: BTreeSet<(String, String)> = wp009_surface_registry()
        .iter()
        .map(|s| (s.method.to_string(), s.route.to_string()))
        .collect();
    let missing: Vec<_> = loom_router_surfaces_from_source()
        .into_iter()
        .filter(|surface| !registry.contains(surface))
        .collect();
    assert!(
        missing.is_empty(),
        "mounted Loom routes missing from wp009_surface_registry: {missing:?}"
    );
}

/// MT-022 NEGATIVE FIXTURE for the scraper itself. A scraper that over-reports
/// is as dangerous as one that under-reports: it pressures the next implementer
/// into registering surfaces the router does not serve, which then fails the
/// live-router probe with 405.
///
/// The LAST route mounted by `api::loom::routes` is
/// `POST /workspaces/:workspace_id/loom/views/definitions/:block_id/results`.
/// Before the `loom_router_body()` slice, its method block ran to end-of-file
/// and absorbed unrelated `get(`/`patch(` tokens from handler code, inventing a
/// GET and a PATCH on that path. Assert the POST is found and the two phantoms
/// are not.
#[test]
fn mtdoc_loom_router_scraper_does_not_invent_methods_past_the_router_body() {
    let scraped = loom_router_surfaces_from_source();
    const LAST_ROUTE: &str = "/workspaces/:workspace_id/loom/views/definitions/:block_id/results";

    assert!(
        scraped.contains(&("POST".to_string(), LAST_ROUTE.to_string())),
        "the router's final route must still be scraped; got {scraped:?}"
    );
    for phantom in ["GET", "PATCH", "PUT", "DELETE"] {
        assert!(
            !scraped.contains(&(phantom.to_string(), LAST_ROUTE.to_string())),
            "{phantom} {LAST_ROUTE} is NOT mounted — the scraper leaked handler-body tokens \
             into the last route's method block"
        );
    }

    // The scraped body must stop at the router. `pub fn routes` is ~290 lines;
    // the file is thousands. If the slice ever regresses to whole-file, this
    // upper bound breaks before the phantom assertions above go quietly stale.
    let body = loom_router_body();
    assert!(
        body.len() < include_str!("../src/api/loom.rs").len() / 4,
        "loom_router_body() must be the router only, got {} bytes",
        body.len()
    );
    assert!(
        !body.contains("async fn "),
        "loom_router_body() must not extend into handler definitions"
    );
    // Anti-vacuous floor: the router really does mount a substantial inventory.
    assert!(
        scraped.len() >= 60,
        "expected the Loom router to mount a substantial route set, scraped {}",
        scraped.len()
    );
}

/// MT-201: pages list + read; an anonymous read works (bootstrap surface) and
/// RETURNS a real, persisted bootstrap receipt.
#[tokio::test]
async fn mt201_pages_list_and_read_with_bootstrap_receipt() {
    let fx = skip_if_no_pg!(fixture().await, "mt201_pages");
    let list: Value = fx
        .http
        .get(format!("{}/usermanual/pages", fx.base))
        .send()
        .await
        .expect("list pages")
        .json()
        .await
        .expect("list json");
    assert_eq!(list["manual_version"], USER_MANUAL_VERSION);
    assert!(
        list["count"].as_u64().unwrap() >= 24,
        "expected the full seeded corpus, got {}",
        list["count"]
    );

    let response = fx
        .http
        .get(format!("{}/usermanual/pages/manual-toc", fx.base))
        .send()
        .await
        .expect("read page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("page json");
    assert_eq!(page["page"]["slug"], "manual-toc");
    assert!(!page["sections"].as_array().unwrap().is_empty());
    assert_eq!(page["bootstrap_identity_used"], true);
    let receipt = page["bootstrap_receipt_event_id"]
        .as_str()
        .expect("receipt id");
    assert!(
        receipt_exists(&fx.kpg, receipt).await,
        "bootstrap receipt {receipt} must be persisted in the EventLedger"
    );

    // Unknown slug: typed 404, not a router fallback.
    let missing = fx
        .http
        .get(format!("{}/usermanual/pages/zzz-no-such-page", fx.base))
        .send()
        .await
        .expect("missing page");
    assert_eq!(missing.status(), 404);
    let body: Value = missing.json().await.expect("404 json");
    assert_eq!(body["error"], "not_found");
}

/// MT-201: search hits pages/sections/tools; an empty query is a typed 400.
#[tokio::test]
async fn mt201_search_finds_pages_and_tools() {
    let fx = skip_if_no_pg!(fixture().await, "mt201_search");
    let found: Value = fx
        .http
        .get(format!("{}/usermanual/search?q=backlinks", fx.base))
        .send()
        .await
        .expect("search")
        .json()
        .await
        .expect("search json");
    assert!(
        found["count"].as_u64().unwrap() > 0,
        "seeded corpus documents backlinks"
    );

    let empty = fx
        .http
        .get(format!("{}/usermanual/search", fx.base))
        .send()
        .await
        .expect("empty search");
    assert_eq!(empty.status(), 400);
    let body: Value = empty.json().await.expect("400 json");
    assert_eq!(body["error"], "bad_request");

    if let Ok(proof_nonce) = std::env::var("HANDSHAKE_MT017_USER_MANUAL_PROOF_NONCE") {
        let navigation: Value = fx
            .http
            .get(format!("{}/usermanual/pages", fx.base))
            .send()
            .await
            .expect("native contract navigation")
            .json()
            .await
            .expect("native contract navigation json");
        let page: Value = fx
            .http
            .get(format!(
                "{}/usermanual/pages/model-runtime-registry-and-loom-degrade",
                fx.base
            ))
            .send()
            .await
            .expect("native contract page")
            .json()
            .await
            .expect("native contract page json");
        assert_eq!(navigation["route_namespace"], "/usermanual");
        assert_eq!(
            page["page"]["slug"],
            "model-runtime-registry-and-loom-degrade"
        );
        assert_eq!(found["query"], "backlinks");

        let artifact_root = std::env::var("HANDSHAKE_ARTIFACTS_DIR")
            .expect("HANDSHAKE_ARTIFACTS_DIR is required when publishing MT-017 UserManual proof");
        let artifact_root = std::fs::canonicalize(artifact_root)
            .expect("HANDSHAKE_ARTIFACTS_DIR must resolve to an existing directory");
        let manifest_dir = std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))
            .expect("backend manifest directory must resolve");
        let worktree_root = manifest_dir
            .parent()
            .and_then(std::path::Path::parent)
            .and_then(std::path::Path::parent)
            .expect("backend crate must live below the worktree src directory");
        let expected_root = std::fs::canonicalize(
            worktree_root
                .parent()
                .expect("worktree must have a parent")
                .join("Handshake_Artifacts"),
        )
        .expect("canonical sibling Handshake_Artifacts directory must exist");
        assert_eq!(
            artifact_root, expected_root,
            "MT-017 UserManual proof must use the canonical sibling Handshake_Artifacts root"
        );
        let path = artifact_root
            .join("handshake-test")
            .join("wp1-final-audit")
            .join("mt017-user-manual-native-contract.json");
        if let Ok(configured_path) = std::env::var("HANDSHAKE_MT017_USER_MANUAL_CONTRACT_ARTIFACT")
        {
            let configured_path = PathBuf::from(configured_path);
            let configured_path = std::fs::canonicalize(
                configured_path
                    .parent()
                    .expect("configured UserManual artifact must have a parent"),
            )
            .expect("configured UserManual artifact parent must resolve")
            .join(
                configured_path
                    .file_name()
                    .expect("configured UserManual artifact must have a file name"),
            );
            assert_eq!(
                configured_path, path,
                "configured MT-017 UserManual artifact must name the canonical contract"
            );
        }
        let bundle = serde_json::json!({
            "schema_id": "hsk.user_manual_native_contract_fixture@1",
            "navigation": navigation,
            "page": page,
            "search": found,
        });
        let pretty = serde_json::to_vec_pretty(&bundle).expect("serialize UserManual contract");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create UserManual proof parent");
        }
        let artifact_sha256 = format!("{:x}", Sha256::digest(&pretty));
        std::fs::write(&path, &pretty).expect("write backend-produced UserManual contract");
        let producer_completed_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock must be after Unix epoch")
            .as_millis() as u64;
        std::fs::write(
            path.with_extension("provenance.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "schema_id": "hsk.mt017_user_manual_contract_provenance@1",
                "proof_nonce": proof_nonce,
                "contract_schema_id": bundle["schema_id"],
                "artifact_sha256": artifact_sha256,
                "producer_test_id": "mt201_search_finds_pages_and_tools",
                "producer_status": "passed_all_backend_assertions",
                "producer_completed_at_unix_ms": producer_completed_at_unix_ms,
            }))
            .expect("serialize UserManual contract provenance"),
        )
        .expect("write UserManual contract provenance");
        println!("REAL_USER_MANUAL_NATIVE_CONTRACT={}", path.display());
    }
}

/// MT-201: the tool catalog resolves by id with failure modes + recovery
/// steps; this is also the MT-112 closure proof — the /knowledge/code/*
/// routes are manual-registered and readable.
#[tokio::test]
async fn mt201_tools_list_and_read_resolve() {
    let fx = skip_if_no_pg!(fixture().await, "mt201_tools");
    let tools: Value = fx
        .http
        .get(format!("{}/usermanual/tools?origin=wp009_surface", fx.base))
        .send()
        .await
        .expect("list tools")
        .json()
        .await
        .expect("tools json");
    let count = tools["count"].as_u64().unwrap();
    assert_eq!(
        count as usize,
        wp009_surface_registry().len(),
        "one wp009 tool entry per registry surface"
    );

    // MT-112 closure: every /knowledge/code/* nav route is a readable entry.
    for surface in wp009_surface_registry()
        .iter()
        .filter(|s| s.route.starts_with("/knowledge/code/"))
    {
        let tool: Value = fx
            .http
            .get(format!(
                "{}/usermanual/tools/{}",
                fx.base, surface.surface_id
            ))
            .send()
            .await
            .expect("read code-nav tool")
            .json()
            .await
            .expect("tool json");
        assert_eq!(tool["tool"]["http_route"], surface.route);
        assert_eq!(tool["tool"]["http_method"], surface.method);
        assert!(!tool["tool"]["common_errors"].as_array().unwrap().is_empty());
        assert!(!tool["tool"]["recovery_steps"]
            .as_array()
            .unwrap()
            .is_empty());
    }

    let missing = fx
        .http
        .get(format!("{}/usermanual/tools/zzz.no.such.tool", fx.base))
        .send()
        .await
        .expect("missing tool");
    assert_eq!(missing.status(), 404);

    // Legacy entries imported (deterministic 10.15.8 mapping).
    let legacy: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=legacy_model_manual&limit=500",
            fx.base
        ))
        .send()
        .await
        .expect("legacy tools")
        .json()
        .await
        .expect("legacy json");
    assert!(
        legacy["count"].as_u64().unwrap() > 50,
        "legacy manifest imported"
    );
}

/// WP-1 MT-002: the ModelLane schema/storage behavior must be discoverable
/// from the in-product UserManual in the same implementation change.
#[tokio::test]
async fn model_lane_schema_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_schema_manual");
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/model-lane-schema", fx.base))
        .send()
        .await
        .expect("read ModelLane schema manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);
    assert_eq!(
        page["page"]["manual_version"], USER_MANUAL_VERSION,
        "page detail must surface the canonical manual_version on its nested page row"
    );

    let legacy: Value = fx
        .http
        .get(format!("{}/usermanual/legacy/model-manual", fx.base))
        .send()
        .await
        .expect("read legacy ModelManual bridge")
        .json()
        .await
        .expect("legacy bridge json");
    let legacy_navigation = legacy["canonical"]["pages"]
        .as_array()
        .expect("legacy pages")
        .iter()
        .find(|row| row["slug"] == "model-lane-navigation")
        .expect("legacy bridge exposes model-lane-navigation");
    let canonical_navigation_response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-navigation",
            fx.base
        ))
        .send()
        .await
        .expect("read canonical ModelLane navigation page");
    assert_eq!(canonical_navigation_response.status(), 200);
    let canonical_navigation: Value = canonical_navigation_response
        .json()
        .await
        .expect("canonical ModelLane navigation json");
    let canonical_navigation_page = &canonical_navigation["page"];
    assert_eq!(canonical_navigation_page["slug"], "model-lane-navigation");
    assert_eq!(
        canonical_navigation_page["manual_version"],
        USER_MANUAL_VERSION
    );
    assert!(
        canonical_navigation_page["content_hash"]
            .as_str()
            .is_some_and(|hash| !hash.trim().is_empty()),
        "canonical same-slug detail must expose a nonempty content_hash"
    );
    assert_eq!(legacy_navigation["slug"], "model-lane-navigation");
    assert!(
        legacy_navigation["manual_version"]
            .as_str()
            .is_some_and(|version| !version.trim().is_empty()),
        "legacy same-slug projection must expose a nonempty manual_version"
    );
    assert!(
        legacy_navigation["content_hash"]
            .as_str()
            .is_some_and(|hash| !hash.trim().is_empty()),
        "legacy same-slug projection must expose a nonempty content_hash"
    );
    assert_eq!(
        legacy_navigation["manual_version"], canonical_navigation_page["manual_version"],
        "legacy ModelManual bridge must carry the same-slug canonical page manual_version"
    );
    assert_eq!(
        legacy_navigation["content_hash"], canonical_navigation_page["content_hash"],
        "legacy ModelManual bridge must carry the same-slug canonical page content_hash"
    );

    for required in [
        "Dexterity",
        "ModelLaneRun",
        "ModelLane",
        "ModelLaneMessage",
        "PostgreSQL",
        "EventLedger",
        "ArtifactStore",
        "CRDT",
        "Flight Recorder",
        "internal_diagnostics",
        "Palmistry",
        "locus_binding_ref",
        "event_ledger_seq",
        "payload_sha256",
        "replay_order_key",
        "recovery_state",
        "promotion_receipt_ref",
        "memory_pack_ref",
        "determinism_mode",
        "budget_summary_ref",
        "SpawnRequest::with_dexterity_launch",
        "SwarmCoordinator::spawn_session",
        "internal_diagnostics is WIRED through the native producer and Problems projection",
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer",
        "dexterity_launch_records_real_swarm_spawn_session_runtime_path",
        "model_lane_schema_rejects_missing_locus_binding_and_idempotency_conflict",
        "model_lane_schema_persists_and_replays_eventledger_rows",
        "model_lane_schema_serializes_competing_terminal_updates",
    ] {
        assert!(
            body.contains(required),
            "model-lane UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let schema_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_schema_pg_tests")
        .expect("manual tool catalog must expose the MT-002 exact proof target");
    assert_eq!(schema_tool["status"], "wired");
    for required in [
        "EventLedger-backed ModelLane rows",
        "runtime spawn_session launch rows",
        "idempotency behavior",
        "replay ordered by event_ledger_seq",
    ] {
        assert!(
            schema_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "schema tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = schema_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneRun",
        "ModelLane",
        "ModelLaneMessage",
        "event_ledger_seq",
        "payload_sha256",
        "replay_order_key",
        "ArtifactStore",
        "CRDT",
        "Flight Recorder",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "schema tool schema_fields must include {required}"
        );
    }
    let common_errors = schema_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("missing locus_binding_ref"));
    assert!(common_errors.contains("idempotency conflict"));
    assert!(common_errors.contains("payload_sha256 is not lowercase sha256 hex"));
    let recovery_steps = schema_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("Run migrations"));
    assert!(recovery_steps.contains("Replay by event_ledger_seq"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

/// WP-1 MT-003: Dexterity launch adapter behavior must be discoverable from
/// the in-product UserManual in the same implementation change.
#[tokio::test]
async fn model_lane_launch_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_launch_manual");
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-launch-adapters",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane launch manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "DexterityLaunchAdapterRegistry",
        "DexterityNormalizedLaunch",
        "SwarmCoordinator::spawn_session",
        "ModelRuntime",
        "CloudLane/BYOK",
        "CliBridge",
        "Operator",
        "SubagentManager",
        "ValidatorRunner",
        "local",
        "BYOK cloud",
        "official CLI",
        "HandshakeNative attached-sandbox",
        "LiveCliSpawner::spawn",
        "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio",
        "GuardedCliChild::terminate_and_collect",
        "ProcessOwnershipLedger START/STOP",
        "terminate/reap-before-STOP",
        "timeout, cancellation, or unwind",
        "durable STOP recording fails",
        "cleanup-pending",
        "human/operator",
        "subagent",
        "validator",
        "direct endpoint",
        "app/src",
        "app/src-tauri",
        "terminal-only",
        "frontend IPC",
        "unmanaged external model-server proof",
        "ModelLaneStore-backed coordinator",
        "SpawnRequest::with_dexterity_launch",
        "Tauri IPC (`kernel_swarm_spawn_session`)",
        "scheduled spin-up",
        "DexterityLaunchContract::attach_to_spawn_request",
        "core-generated Dexterity contract",
        "byok_cloud_provider",
        "model launch startup fails closed",
        "cancellation token boundary",
        "reclaim policy",
        "terminal status mapping",
        "startup_failure_code",
        "process_ownership_ref",
        "no_os_process_reason_ref",
        "unsupported tool capability",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
        "SwarmCoordinator::revoke_cloud_consent_receipt",
        "Cancelled/CX-MM-007",
        "context switched in any one scope dimension observes absence",
        "original exact owner retains scoped audit and recovery access",
        "Flight Recorder/EventLedger",
        "Palmistry",
        "internal_diagnostics",
        "model_lane_launch_all_lane_kinds_through_rust_registry",
        "model_lane_launch_rejects_direct_endpoint_frontend_tauri_and_terminal_bypass",
        "model_lane_launch_cancellation_reclaim_contracts_all_lane_kinds",
        "model_lane_launch_records_factory_failure_through_swarm_coordinator",
        "production_builder_wires_model_lane_store_for_failed_dexterity_launch",
        "model_lane_launch_rejects_ready_transition_before_persistence_commit",
        "model_lane_launch_cancel_session_records_terminal_model_lane_state",
        "model_lane_launch_reaper_records_terminal_state_before_teardown",
        "explicit_failed_terminate_leaves_start_open_without_stop",
        "failed_termination_with_never_eof_pipe_returns_within_cleanup_deadline",
        "continuous_output_cannot_starve_live_timeout_polling",
        "continuous_output_cannot_starve_live_cancellation_polling",
        "dexterity_launch_records_real_swarm_spawn_session_runtime_path",
        "model_lane_launch_user_manual_entry_is_current",
        "model_lane_schema_user_manual_entry_is_current",
        "HANDSHAKE_TEST_GGUF_PATH",
        "llama-cpp-runtime-engine",
        "same-lane-id mutable projection collision",
        "OsKeychainSecretsVault",
        "live one-token generation",
        "missing prerequisite fails the proof explicitly",
        "docs/model-manual",
        "app/MODEL_MANUAL.md",
        "npm/JavaScript proof",
        "reference-only and never launch authority",
        "coordinator-owned no-OS launch records",
        "durable cancellation terminal state",
        "no Ready/runtime exposure before ModelLane persistence",
        "retryable terminal intent before runtime teardown",
        "cleanup owner's observable progress",
        "coordinator-owned task",
        "claim.attempt",
        "restart_generation",
        "generation-scoped",
        "ac9_cancel_terminalizes_parallel_siblings_and_live_sessions",
        "ac9_crash_after_persisted_spawn_intent_recovers_with_new_fence_and_compensation",
        "per-lane terminal serialization",
        "No-OS caller receipts are minted from a live",
        "not offline bearer tokens",
        "terminal-failure://dexterity/<lane_id>",
    ] {
        assert!(
            body.contains(required),
            "model-lane launch UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_schema_pg_tests"));
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("official_cli_attached_lifecycle_tests"));

    let launch_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_launch_tests")
        .expect("launch tool entry");
    assert_eq!(launch_tool["status"], "wired");
    assert!(launch_tool["description"]
        .as_str()
        .unwrap()
        .contains("runtime-owned launch paths"));
    for required in [
        "caller receipts",
        "no Ready/runtime exposure before ModelLane persistence",
        "durable cancellation terminal state",
        "lease-reaper terminal persistence before teardown",
        "missing-contract bypass rejection",
        "terminal-failure refs for runtime failed state",
        "retryable terminal intent before runtime teardown",
        "per-lane terminal serialization",
        "EventLedger stream-backed rows",
    ] {
        assert!(
            launch_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "launch tool expected_output must mention {required}"
        );
    }

    let official_cli_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "official_cli_attached_lifecycle_tests")
        .expect("official CLI attached lifecycle tool entry");
    assert_eq!(official_cli_tool["status"], "wired");
    for required in [
        "LiveCliSpawner::spawn",
        "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio",
        "GuardedCliChild::terminate_and_collect",
        "success, failure, timeout, cancellation, and unwind",
        "STOP is recorded only after termination and reap succeed",
        "unreaped termination leaves the durable START open with no fabricated STOP",
    ] {
        assert!(
            official_cli_tool["expected_output"]
                .as_str()
                .expect("official CLI expected output")
                .contains(required),
            "official CLI lifecycle tool expected_output must mention {required}"
        );
    }
    let official_cli_schema_fields = official_cli_tool["schema_fields"]
        .as_array()
        .expect("official CLI schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for required in [
        "LiveCliSpawner::spawn",
        "HandshakeNativeSandboxAdapter::spawn_attached_with_stdio",
        "GuardedCliChild::terminate_and_collect",
        "ProcessOwnershipLedger START",
        "ProcessOwnershipLedger STOP",
    ] {
        assert!(
            official_cli_schema_fields.contains(required),
            "official CLI lifecycle tool schema_fields must contain {required}"
        );
    }
    let official_cli_errors = official_cli_tool["common_errors"]
        .as_array()
        .expect("official CLI common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(official_cli_errors.contains("termination or reap fails"));
    assert!(official_cli_errors.contains("STOP is fabricated"));
    let official_cli_recovery = official_cli_tool["recovery_steps"]
        .as_array()
        .expect("official CLI recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(official_cli_recovery.contains("preserve it as cleanup-pending evidence"));
    assert!(official_cli_recovery.contains("do not synthesize STOP"));
    let schema_fields: std::collections::BTreeSet<_> = launch_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "event_ledger_stream_id",
        "DexterityNoOsLaunchCaller",
        "model_lane_terminal",
        "Flight Recorder",
        "EventLedger",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "launch tool schema_fields must include {required}"
        );
    }
    let common_errors = launch_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("direct endpoint launch bypass"));
    assert!(common_errors.contains("ModelLaneStore-backed coordinator"));
    assert!(common_errors.contains("Ready transition before ModelLane persistence commit"));
    assert!(common_errors.contains("terminal state write failure before runtime teardown"));
    assert!(common_errors.contains("stale no-OS caller receipt after authority session removal"));
    let recovery_steps = launch_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("DexterityLaunchAdapterRegistry"));
    assert!(recovery_steps.contains("SpawnRequest::with_dexterity_launch"));
    assert!(recovery_steps.contains("ModelLaneStore"));
    assert!(recovery_steps.contains("authorize from a live Ready/Generating authority session"));
    assert!(recovery_steps.contains("live handle still exists"));
    assert!(recovery_steps.contains("terminal writes serialize by lane_id"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

/// WP-1 MT-004: Dexterity routing and promotion behavior must be discoverable
/// from the in-product UserManual in the same implementation change.
#[tokio::test]
async fn model_lane_promotion_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_promotion_manual");
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/model-lane-promotion", fx.base))
        .send()
        .await
        .expect("read ModelLane promotion manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity",
        "ModelLanePromotionDecision",
        "NewModelLanePromotionDecision",
        "hsk.model_lane_promotion_decision@1",
        "model_lane_promotion_decision",
        "local_first",
        "cloud_review",
        "cloud_plan_local_execute",
        "parallel_debate",
        "validator_lane",
        "operator_lane",
        "expired",
        "skipped",
        "unsupported",
        "advisory -> promotion_requested -> pending_policy -> pending_approval -> approved -> executing -> executed",
        "advisory -> promotion_requested -> pending_policy -> denied",
        "target_role",
        "target_session",
        "correlation_id",
        "requires_ack",
        "ack_for",
        "promotion_decision_id",
        "promotion_gate_ref",
        "promotion_receipt_ref",
        "promoted_artifact_ref",
        "promoted_artifact_sha256",
        "promoted_artifact_version",
        "final_state",
        "expected_event_ledger_version",
        "current_event_ledger_version",
        "base_snapshot_ref",
        "current_base_snapshot_ref",
        "state_vector",
        "current_state_vector",
        "schema_id",
        "deterministic tie-break rule",
        "canonical_hash_basis",
        "canonical_decision_hash",
        "AggregateVersionMismatch",
        "SchemaMismatch",
        "StaleBase",
        "StaleStateVector",
        "InputRefMismatch",
        "DirectAuthorityMutation",
        "MissingPromotionAuthority",
        "MissingPromotedArtifactBinding",
        "ModelLaneAuthority::Promoted",
        "PromotionGate resolution",
        "ModelLaneStore::new_scoped",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
        "caller payloads cannot select or widen them",
        "foreign account, Principal, session, AccessSpace, or workspace",
        "before any promoted-message or EventLedger authority mutation",
        "mixed resources from incompatible scopes",
        "PromotionGate never widens a derived artifact",
        "Proposal",
        "Critique",
        "Recovery",
        "ContextBundle",
        "recovery_hint_ref",
        "Flight Recorder/EventLedger",
        "internal_diagnostics",
        "Palmistry",
        "DEFERRED-with-reason",
        "model_lane_promotion_appends_eventledger_and_replays_decision",
        "model_lane_promotion_rejects_stale_base_schema_mismatch_and_direct_mutation",
        "model_lane_promotion_reordered_inputs_keep_same_decision_hash",
        "model_lane_promotion_preserves_exact_scope_and_denies_foreign_or_mixed_sources",
        "model_lane_promotion_user_manual_entry_is_current",
        "no SQLite",
        "app/src-tauri",
        "TypeScript",
    ] {
        assert!(
            body.contains(required),
            "model-lane promotion UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_schema_pg_tests"));
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("model_lane_promotion_pg_tests"));

    let promotion_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_promotion_pg_tests")
        .expect("promotion tool entry");
    assert_eq!(promotion_tool["status"], "wired");
    assert!(promotion_tool["description"]
        .as_str()
        .unwrap()
        .contains("advisory-to-authority promotion decisions"));
    for required in [
        "EventLedger-backed ModelLanePromotionDecision rows",
        "typed approved/denied state_history",
        "final_state",
        "DB-derived CRDT base/state-vector denials",
        "schema and aggregate-version denials",
        "phantom input-ref denial",
        "exact promotion_decision_id and promoted artifact binding",
        "direct authority mutation rejection",
        "duplicate idempotency conflict",
        "typed message routing",
        "canonical decision hash stable across reordered input refs",
    ] {
        assert!(
            promotion_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "promotion tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = promotion_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneRoutingPolicy",
        "ModelLaneRoutingMetadata",
        "ModelLanePromotionDecision",
        "NewModelLanePromotionDecision",
        "hsk.model_lane_promotion_decision@1",
        "target_role",
        "target_session",
        "correlation_id",
        "requires_ack",
        "ack_for",
        "canonical_hash_basis",
        "canonical_decision_hash",
        "final_state",
        "expected_event_ledger_version",
        "current_event_ledger_version",
        "base_snapshot_ref",
        "current_base_snapshot_ref",
        "state_vector",
        "current_state_vector",
        "schema_id",
        "promotion_decision_id",
        "promotion_gate_ref",
        "promotion_receipt_ref",
        "promoted_artifact_ref",
        "promoted_artifact_sha256",
        "promoted_artifact_version",
        "event_ledger_seq",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "promotion tool schema_fields must include {required}"
        );
    }
    let common_errors = promotion_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("AggregateVersionMismatch"));
    assert!(common_errors.contains("SchemaMismatch"));
    assert!(common_errors.contains("StaleBase"));
    assert!(common_errors.contains("StaleStateVector"));
    assert!(common_errors.contains("InputRefMismatch"));
    assert!(common_errors.contains("DirectAuthorityMutation"));
    assert!(common_errors.contains("MissingPromotedArtifactBinding"));
    assert!(common_errors.contains("PromotionGate resolution"));
    assert!(common_errors.contains("idempotency conflict"));
    let recovery_steps = promotion_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::replay_promotion_decisions"));
    assert!(recovery_steps.contains("current_base_snapshot_ref/current_state_vector"));
    assert!(recovery_steps.contains("model_lane_schema_registry"));
    assert!(recovery_steps.contains("kernel_event_ledger aggregate version"));
    assert!(recovery_steps.contains("model-lane-message:// ref exists"));
    assert!(recovery_steps.contains("Never write ModelLaneAuthority::Promoted directly"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

/// WP-1 MT-005: Dexterity ContextBundle handoff behavior must be
/// discoverable from the in-product UserManual in the same implementation
/// change.
#[tokio::test]
async fn model_lane_context_bundle_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_context_bundle_manual");
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-context-bundle-handoff",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane ContextBundle manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity",
        "ModelLaneContextBundleHandoff",
        "NewModelLaneContextBundleArtifactBinding",
        "ModelLaneContextBundleArtifactBindingRecord",
        "NewModelLaneContextBundleHandoff",
        "ModelLaneContextBundleHandoffRecord",
        "ModelLaneDownstreamContextBundle",
        "ModelLaneCrdtHandoffMetadata",
        "ModelLaneLoomHandoffRef",
        "ModelLaneMemoryPackHandoffRef",
        "ModelLaneStore::record_context_bundle_artifact_binding",
        "ModelLaneStore::record_context_bundle_handoff",
        "ModelLaneStore::replay_context_bundle_handoffs",
        "ModelLaneStore::consume_context_bundle_for_downstream",
        "SwarmCoordinator::context_bundle_for_downstream_lane",
        "SwarmCoordinator::invoke_downstream_context_bundle",
        "ModelAdapterRequest",
        "model_lane_context_bundle_id_for_handoff",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "model_lane_context_bundle_artifacts",
        "model_lane_context_bundle_handoff",
        "ARTIFACT_STORED",
        "CONTEXT_BUNDLE_RECORDED",
        "context_bundle_id",
        "context_bundle_hash",
        "artifact_binding_hash",
        "artifact_manifest_ref",
        "artifact_payload_ref",
        "payload_json",
        "downstream_lane_id",
        "work_packet_id",
        "micro_task_id",
        "task_board_id",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "ModelLaneStore::new_scoped",
        "server-derived scope is authority",
        "default-denied at SQL and post-decode boundaries",
        "session or AccessSpace switch cannot retarget",
        "to_kernel_context_bundle",
        "CTX-<hash>",
        "selected",
        "rejected",
        "unresolved",
        "superseded",
        "source_message_id",
        "artifact_ref",
        "artifact_sha256",
        "content_hash",
        "hsk.model_lane_crdt_payload@1",
        "update_bytes_ref",
        "update_sha256",
        "state_vector",
        "base_snapshot_ref",
        "materialized_projection_hash",
        "knowledge_crdt_agent_lane_leases",
        "ModelLaneCrdtAuthorityBinding",
        "lease_id",
        "lease_correlation_id",
        "lease_scope_kind",
        "lease_scope_id",
        "lease_claimed_at_utc",
        "lease_expires_at_utc",
        "lease_admitted_at_utc",
        "released_at_utc IS NULL",
        "claimed_at_utc <= lease_admitted_at_utc < expires_at_utc",
        "MODEL_RESPONSE_RECORDED",
        "multiple covering workspace/document matches",
        "later release or natural expiry",
        "historical replay",
        "stored-authority validators",
        "Old rows with a missing binding are rejected",
        "CONTEXT_BUNDLE_RECORDED",
        "recompute its canonical",
        "transaction advisory-lock domain",
        "workspace:<workspace_id>",
        "crdt_document:<crdt_document_id>",
        "second covering claim",
        "mutable projection/EventLedger disagreement",
        "replay_metadata",
        "replay_order_key",
        "dependency_update_ids",
        "schema_version",
        "forged values fail closed before any handoff row is written",
        "Yjs-compatible",
        "yjs_update_v1",
        "promotion-gate://model-lane-message/<source_message_id>",
        "eventledger://<update_event_id>",
        "authority_effect = advisory_only",
        "event_ledger_evidence_ref",
        "flight_recorder_evidence_ref",
        "memory_pack_ref",
        "memory_pack_hash",
        "scope_tag",
        "review_status",
        "cloud_safe",
        "classification",
        "projection_ref",
        "local_only_context",
        "hidden provider memory",
        "not replayable",
        "cloud_safe = false",
        "ArtifactStore/EventLedger authority",
        "review_status must be reviewed",
        "projection_ref",
        "memory_pack_refs exceeds bounded FEMS limit",
        "loom_refs exceeds bounded limit",
        "Flight Recorder/EventLedger",
        "internal_diagnostics",
        "Palmistry",
        "DEFERRED-with-reason",
        "model_lane_context_bundle_persists_selection_state_and_replays",
        "model_lane_context_bundle_missing_artifact_ref_fails_closed",
        "model_lane_context_bundle_crdt_state_vector_and_loom_refs_are_replayable",
        "model_lane_context_bundle_user_manual_entry_is_current",
        "no SQLite",
        "prompt-only",
        "hidden-memory",
    ] {
        assert!(
            body.contains(required),
            "model-lane ContextBundle UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_schema_pg_tests"));
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("model_lane_promotion_pg_tests"));
    assert!(tool_ids.contains("model_lane_context_bundle_pg_tests"));

    let handoff_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_context_bundle_pg_tests")
        .expect("ContextBundle tool entry");
    assert_eq!(handoff_tool["status"], "wired");
    assert!(handoff_tool["description"]
        .as_str()
        .unwrap()
        .contains("model-to-model ContextBundle handoff persistence"));
    for required in [
        "EventLedger-backed ModelLaneContextBundleArtifactBindingRecord",
        "ARTIFACT_STORED payload stamping",
        "model_lane_context_bundle_artifacts authority rows",
        "artifact_manifest_ref/artifact_payload_ref/payload_json/artifact_binding_hash",
        "EventLedger-backed ModelLaneContextBundleArtifactBindingRecord and ModelLaneContextBundleHandoff rows",
        "downstream-only ModelLaneDownstreamContextBundle consumption",
        "ModelLaneStore::consume_context_bundle_for_downstream",
        "SwarmCoordinator::context_bundle_for_downstream_lane",
        "SwarmCoordinator::invoke_downstream_context_bundle adapter invocation",
        "to_kernel_context_bundle conversion",
        "ContextBundle V1 CTX-<hash> identity",
        "selected/rejected/unresolved/superseded selection states",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "fail-closed missing source",
        "artifact_ref/artifact_sha256/content_hash mismatch",
        "ArtifactStore/EventLedger authority",
        "cloud-safe FEMS MemoryPack enforcement",
        "local_only_context cloud rejection",
        "review_status reviewed, operator_reviewed, or validator_reviewed",
        "hidden provider/session memory rejection including projection_ref and normalized hidden-memory URI checks",
        "memory_pack_refs exceeds bounded FEMS limit",
        "CRDT state_vector/base_snapshot_ref/update_bytes_ref validation",
        "Yjs-compatible format yjs_update_v1 only",
        "Loom event_ledger_evidence_ref and flight_recorder_evidence_ref replay",
        "loom_refs exceeds bounded limit",
        "duplicate idempotency returning the original context_bundle_hash",
        "exact five-field resource scope retained on ContextBundle and derived artifact refs",
        "SQL plus post-decode default-deny across owner/Principal/authenticated session/AccessSpace/workspace switches",
    ] {
        assert!(
            handoff_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "ContextBundle tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = handoff_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneContextBundleArtifactBindingRecord",
        "NewModelLaneContextBundleArtifactBinding",
        "ModelLaneContextBundleHandoffRecord",
        "NewModelLaneContextBundleHandoff",
        "ModelLaneDownstreamContextBundle",
        "ModelLaneCrdtHandoffMetadata",
        "ModelLaneLoomHandoffRef",
        "ModelLaneMemoryPackHandoffRef",
        "hsk.model_lane_context_bundle_artifact@1",
        "hsk.model_lane_context_bundle_handoff@1",
        "model_lane_context_bundle_artifacts",
        "context_bundle_id",
        "context_bundle_hash",
        "artifact_binding_hash",
        "artifact_manifest_ref",
        "artifact_payload_ref",
        "payload_json",
        "downstream_lane_id",
        "SwarmCoordinator::invoke_downstream_context_bundle",
        "ModelAdapterRequest",
        "source_message_id",
        "artifact_ref",
        "artifact_sha256",
        "content_hash",
        "work_packet_id",
        "micro_task_id",
        "task_board_id",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "to_kernel_context_bundle",
        "CTX-<hash>",
        "selected",
        "rejected",
        "unresolved",
        "superseded",
        "update_bytes_ref",
        "update_sha256",
        "state_vector",
        "base_snapshot_ref",
        "materialized_projection_hash",
        "replay_metadata",
        "replay_order_key",
        "dependency_update_ids",
        "schema_version",
        "yjs_update_v1",
        "validation_runner_ref",
        "authority_effect",
        "event_ledger_evidence_ref",
        "flight_recorder_evidence_ref",
        "memory_pack_ref",
        "memory_pack_hash",
        "scope_tag",
        "review_status",
        "cloud_safe",
        "classification",
        "projection_ref",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "ContextBundle tool schema_fields must include {required}"
        );
    }
    let common_errors = handoff_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("artifact binding must exist"));
    assert!(common_errors.contains("artifact_payload_ref must match artifact_ref"));
    assert!(common_errors.contains("payload_json sha256 must match content_hash"));
    assert!(common_errors.contains("source_message_id is not replayable"));
    assert!(common_errors.contains("handoff.artifact_ref must match source.payload_ref"));
    assert!(common_errors.contains("handoff.artifact_sha256 must match source.payload_sha256"));
    assert!(common_errors.contains("ArtifactStore/EventLedger authority"));
    assert!(common_errors.contains("downstream_lane_id is required"));
    assert!(common_errors.contains("work_packet_id is required"));
    assert!(common_errors.contains("micro_task_id is required"));
    assert!(common_errors.contains("task_board_id is required"));
    assert!(common_errors.contains("cloud_safe"));
    assert!(common_errors.contains("local_only_context"));
    assert!(common_errors.contains("hidden provider/session memory"));
    assert!(common_errors.contains("projection_ref"));
    assert!(common_errors.contains("review_status"));
    assert!(common_errors.contains("memory_pack_refs exceeds bounded FEMS limit"));
    assert!(common_errors.contains("crdt_payload.update_bytes_ref"));
    assert!(common_errors.contains("Yjs-compatible format yjs_update_v1"));
    assert!(common_errors.contains("advisory_only"));
    assert!(common_errors.contains("loom_refs exceeds bounded limit"));
    assert!(common_errors.contains("missing or mismatched exact resource scope is default-denied"));
    let recovery_steps = handoff_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::record_context_bundle_artifact_binding"));
    assert!(recovery_steps.contains("model_lane_context_bundle_artifacts.payload_json"));
    assert!(recovery_steps.contains("ModelLaneStore::replay_context_bundle_handoffs"));
    assert!(recovery_steps.contains("ModelLaneStore::consume_context_bundle_for_downstream"));
    assert!(recovery_steps.contains("SwarmCoordinator::context_bundle_for_downstream_lane"));
    assert!(recovery_steps.contains("SwarmCoordinator::invoke_downstream_context_bundle"));
    assert!(recovery_steps.contains("ModelAdapterRequest.context_bundle"));
    assert!(recovery_steps.contains("ModelLaneDownstreamContextBundle"));
    assert!(recovery_steps.contains("to_kernel_context_bundle"));
    assert!(recovery_steps.contains("CTX-<hash>"));
    assert!(recovery_steps.contains("kernel_event_ledger"));
    assert!(recovery_steps.contains("source ModelLaneMessage first"));
    assert!(recovery_steps.contains(
        "original server-derived owner/Principal/authenticated session/AccessSpace/workspace context"
    ));
    assert!(recovery_steps.contains("artifact_ref/artifact_sha256/content_hash"));
    assert!(recovery_steps.contains("cloud_safe = true"));
    assert!(recovery_steps.contains("classification other than local_only_context"));
    assert!(recovery_steps.contains("update_bytes_ref, state_vector, and base_snapshot_ref"));
    assert!(recovery_steps.contains("EventLedger and Flight Recorder evidence refs"));
    assert!(recovery_steps.contains("direct Flight Recorder event emission"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

/// WP-1 MT-006: Dexterity cloud ProjectionPlan/ConsentReceipt policy must be
/// discoverable from the in-product UserManual in the same implementation
/// change.
#[tokio::test]
async fn cloud_model_lane_policy_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "cloud_model_lane_policy_manual");
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-cloud-projection-consent",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane cloud policy manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity",
        "ModelLaneCloudProjectionPlanRecord",
        "NewModelLaneCloudProjectionPlan",
        "ModelLaneCloudConsentReceiptRecord",
        "NewModelLaneCloudConsentReceipt",
        "ModelLaneStore::record_cloud_projection_plan",
        "ModelLaneStore::record_cloud_consent_receipt",
        "ModelLaneStore::replay_cloud_consent_authority",
        "ModelLaneStore::preflight_cloud_spawn_request",
        "SwarmCoordinator::revoke_cloud_consent_receipt",
        "SwarmCoordinator::spawn_session",
        "factory.create",
        "hsk.model_lane_cloud_projection_plan@2",
        "hsk.model_lane_cloud_consent_receipt@2",
        "hsk.model_lane_cloud_consent_denial@1",
        "model_lane_cloud_projection_plans",
        "model_lane_cloud_consent_receipts",
        "model_lane_cloud_consent_denial",
        "model_lane_terminal",
        "CX-MM-007",
        "consent_status",
        "provider_call_attempted = false",
        "partial_authority_state_created",
        "operator-chat cloud launches precompute deterministic ArtifactStore refs",
        "/operator-chat/cloud/single-run/grant-launch",
        "/operator-chat/cloud/single-run/revoke",
        "checks for `single_lane` cover",
        "Checks for `single_run` cover",
        "reject lane-bound identity fields",
        "cloud-input.json",
        "cloud-projection-payload.json",
        "after `spawn_session` records the ModelLaneRun",
        "before output capture",
        "ProjectionPlan",
        "ConsentReceipt",
        "projection_plan_hash",
        "run_id",
        "lane_id",
        "model_session_id",
        "provider_kind",
        "requested_model_id",
        "scope_hash",
        "retention",
        "export",
        "fan-out",
        "EventLedger",
        "Direct Flight Recorder event emission",
        "FR-EVT-CLOUD",
        "internal_diagnostics",
        "Palmistry",
        "DEFERRED-with-reason",
        "cloud_projection_and_consent_receipts_persist_and_replay",
        "cloud_lane_rejects_missing_expired_mismatched_and_revoked_consent",
        "cloud_consent_revocation_cancels_pending_lanes_with_eventledger_evidence",
        "cloud_model_lane_policy_user_manual_entry_is_current",
        "SQLite",
        "prompt-only",
        "synthetic refs",
        "frontend/Tauri launch authority",
    ] {
        assert!(
            body.contains(required),
            "model-lane cloud policy UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_schema_pg_tests"));
    assert!(tool_ids.contains("model_lane_launch_tests"));
    assert!(tool_ids.contains("model_lane_promotion_pg_tests"));
    assert!(tool_ids.contains("model_lane_context_bundle_pg_tests"));
    assert!(tool_ids.contains("cloud_model_lane_policy_pg_tests"));

    let cloud_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "cloud_model_lane_policy_pg_tests")
        .expect("cloud policy tool entry");
    assert_eq!(cloud_tool["status"], "wired");
    assert!(cloud_tool["description"]
        .as_str()
        .unwrap()
        .contains("ProjectionPlan/ConsentReceipt persistence"));
    for required in [
        "EventLedger-backed ModelLaneCloudProjectionPlanRecord and ModelLaneCloudConsentReceiptRecord rows",
        "model_lane_cloud_projection_plans and model_lane_cloud_consent_receipts",
        "hsk.model_lane_cloud_projection_plan@2",
        "hsk.model_lane_cloud_consent_receipt@2",
        "hsk.model_lane_cloud_consent_denial@1",
        "ModelLaneStore::replay_cloud_consent_authority",
        "single-lane cloud launch allowed only when",
        "single-run launch allowed only when durable run-scoped authority matches run_id",
        "model_lane_cloud_consent_denial",
        "provider_call_attempted = false",
        "SwarmCoordinator::spawn_session preflight blocks before factory.create",
        "ModelLaneAuthority::Promoted rejects without approved PromotionGate",
        "SwarmCoordinator::revoke_cloud_consent_receipt",
        "model_lane_terminal EventLedger evidence",
    ] {
        assert!(
            cloud_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "cloud policy tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = cloud_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "NewModelLaneCloudProjectionPlan",
        "ModelLaneCloudProjectionPlanRecord",
        "NewModelLaneCloudConsentReceipt",
        "ModelLaneCloudConsentReceiptRecord",
        "ModelLaneCloudConsentAuthorityReplay",
        "ModelLaneStore::record_cloud_projection_plan",
        "ModelLaneStore::record_cloud_consent_receipt",
        "ModelLaneStore::replay_cloud_consent_authority",
        "ModelLaneStore::preflight_cloud_spawn_request",
        "SwarmCoordinator::revoke_cloud_consent_receipt",
        "hsk.model_lane_cloud_projection_plan@2",
        "hsk.model_lane_cloud_consent_receipt@2",
        "hsk.model_lane_cloud_consent_denial@1",
        "model_lane_cloud_projection_plans",
        "model_lane_cloud_consent_receipts",
        "model_lane_cloud_consent_denial",
        "model_lane_terminal",
        "CX-MM-007",
        "consent_status",
        "provider_call_attempted",
        "projection_plan_hash",
        "scope_hash",
        "retention_policy",
        "export_posture",
        "fan_out_targets",
        "redaction_policy_ref",
        "user_manual_behavior_ref",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "cloud policy tool schema_fields must include {required}"
        );
    }
    let common_errors = cloud_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("ProjectionPlan is not durable"));
    assert!(common_errors.contains("ConsentReceipt is not durable"));
    assert!(common_errors.contains("validity window is not current"));
    assert!(common_errors.contains("ConsentReceipt is revoked"));
    assert!(common_errors.contains("scope, retention, export, and fan-out"));
    assert!(common_errors.contains("cloud lane launch denied before provider call"));
    assert!(common_errors.contains("PromotionGate resolution"));
    assert!(common_errors.contains("hidden provider/session memory"));
    assert!(common_errors.contains("idempotency conflict"));
    let recovery_steps = cloud_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::record_cloud_projection_plan"));
    assert!(recovery_steps.contains("kernel_event_ledger"));
    assert!(recovery_steps.contains("ModelLaneStore::record_cloud_consent_receipt"));
    assert!(recovery_steps.contains("For single_lane verify projection_plan_hash/run_id/lane_id/model_session_id/provider_kind/requested_model_id/scope_hash/retention/export/fan_out_targets"));
    assert!(recovery_steps.contains("for single_run verify projection_plan_hash/run_id/scope_hash/retention/export/fan_out_targets"));
    assert!(recovery_steps.contains("confirm no lane-bound identity is present"));
    assert!(recovery_steps.contains("model_lane_cloud_consent_denial"));
    assert!(recovery_steps.contains("provider_call_attempted = false"));
    assert!(recovery_steps.contains("SwarmCoordinator::revoke_cloud_consent_receipt"));
    assert!(recovery_steps.contains("failstate_code CX-MM-007"));
    assert!(recovery_steps.contains("ModelLaneAuthority::Advisory"));
    assert!(recovery_steps.contains("PromotionGate decision"));
    assert!(recovery_steps.contains("EventLedger rows"));
    assert!(
        recovery_steps.contains("direct Flight Recorder event emission is DEFERRED-with-reason")
    );
    assert!(recovery_steps.contains("FR-EVT-CLOUD"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

/// WP-1 MT-007: Dexterity recovery must be discoverable from the in-product
/// UserManual in the same implementation change.
#[tokio::test]
async fn model_lane_recovery_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_recovery_manual");
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/model-lane-recovery", fx.base))
        .send()
        .await
        .expect("read ModelLane recovery manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity",
        "ModelLaneStore::recover_run_after_restart",
        "hsk.model_lane_recovery_checkpoint@1",
        "hsk.model_lane_recovery_event@1",
        "hsk.model_lane_lease@1",
        "hsk.model_lane_diagnostic_tier@1",
        "hsk.model_lane_mt_runtime_status@1",
        "model_lane_recovery_checkpoints",
        "model_lane_recovery_events",
        "model_lane_leases",
        "model_lane_diagnostic_tier_statuses",
        "model_lane_mt_runtime_statuses",
        "model_lane_context_bundle_artifacts",
        "kernel_event_ledger",
        "last_event_ledger_seq",
        "last_message_id",
        "idempotency_scope",
        "recovery_state",
        "ArtifactStore",
        "CRDT",
        "expired active lease orphans",
        "orphan_detected",
        "current lease authority",
        "post-checkpoint lease",
        "CX-MM-006",
        "CX-MM-009",
        "Flight Recorder/EventLedger evidence alone must fail",
        "internal_diagnostics",
        "Palmistry",
        "DEFERRED-with-reason",
        "model_lane_recovery_replays_from_postgres_eventledger_checkpoint",
        "model_lane_recovery_includes_current_leases_but_bounds_replay_adjunct_state",
        "model_lane_recovery_rejects_corrupt_checkpoint_and_event_seq_gap",
        "model_lane_recovery_restores_mt_runtime_status_refs_after_restart",
        "diagnostic_tier_record_rejects_flight_recorder_only_evidence",
        "model_lane_recovery_rejects_missing_payload_stale_crdt_and_duplicate_idempotency",
        "model_lane_recovery_uses_eventledger_checkpoint_authority_over_mutable_row",
        "model_lane_recovery_rejects_post_checkpoint_payload_and_crdt_repairs",
        "five-dimensional resource scope",
        "RESOURCE_SCOPE_AUTHORITY_UNAVAILABLE",
        "stale successor session",
        "never converts an account-scoped parent into an unscoped legacy row",
        "model_lane_recovery_preserves_exact_scope_and_denies_foreign_stale_replay",
        "recovery_route_is_exact_scoped_and_revoked_authority_is_absent_shaped",
        "model_lane_recovery_user_manual_entry_is_current",
    ] {
        assert!(
            body.contains(required),
            "model-lane recovery UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("model_lane_recovery_pg_tests"));

    let recovery_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_recovery_pg_tests")
        .expect("recovery tool entry");
    assert_eq!(recovery_tool["status"], "wired");
    assert!(recovery_tool["description"]
        .as_str()
        .unwrap()
        .contains("checkpoint/EventLedger recovery"));
    for required in [
        "EventLedger-backed ModelLaneRecoveryCheckpointRecord",
        "ModelLaneRecoveryEventRecord",
        "ModelLaneLeaseRecord",
        "ModelLaneDiagnosticTierStatusRecord",
        "ModelLaneMtRuntimeStatusRecord",
        "checkpoint-bounded replay through ModelLaneStore::recover_run_after_restart",
        "model_lane_context_bundle_artifacts plus kernel_event_ledger",
        "CRDT base/state-vector validation",
        "failed cloud consent denial receipts",
        "active versus expired lease classification",
        "divergent idempotency rejected",
        "CX-MM-006 and CX-MM-009 failure paths",
        "Flight Recorder-only HBR-INT-009 evidence rejected",
    ] {
        assert!(
            recovery_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "recovery tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = recovery_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "NewModelLaneRecoveryCheckpoint",
        "ModelLaneRecoveryCheckpointRecord",
        "NewModelLaneRecoveryEvent",
        "ModelLaneRecoveryEventRecord",
        "NewModelLaneLease",
        "ModelLaneLeaseRecord",
        "NewModelLaneDiagnosticTierStatus",
        "ModelLaneDiagnosticTierStatusRecord",
        "NewModelLaneMtRuntimeStatus",
        "ModelLaneMtRuntimeStatusRecord",
        "ModelLaneStore::recover_run_after_restart",
        "hsk.model_lane_recovery_checkpoint@1",
        "hsk.model_lane_recovery_event@1",
        "hsk.model_lane_lease@1",
        "hsk.model_lane_diagnostic_tier@1",
        "hsk.model_lane_mt_runtime_status@1",
        "model_lane_recovery_checkpoints",
        "model_lane_recovery_events",
        "model_lane_leases",
        "model_lane_diagnostic_tier_statuses",
        "model_lane_mt_runtime_statuses",
        "kernel_event_ledger",
        "model_lane_context_bundle_artifacts",
        "CX-MM-006",
        "CX-MM-009",
        "internal_diagnostics",
        "Palmistry",
    ] {
        assert!(
            schema_fields.contains(required),
            "recovery tool schema_fields must include {required}"
        );
    }
    let common_errors = recovery_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("missing_payload_authority"));
    assert!(common_errors.contains("event_ledger_sequence_gap"));
    assert!(common_errors.contains("stale_crdt_base"));
    assert!(common_errors.contains("orphaned_subagent"));
    assert!(common_errors.contains("idempotency conflict"));
    assert!(common_errors.contains("FlightRecorder-only"));
    let recovery_steps = recovery_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::recover_run_after_restart"));
    assert!(recovery_steps.contains("checkpoint high-watermark"));
    assert!(recovery_steps.contains("recovery_events replay_order_seq"));
    assert!(recovery_steps.contains("model_lane_context_bundle_artifacts"));
    assert!(recovery_steps.contains("kernel_event_ledger"));
    assert!(recovery_steps.contains("CRDT"));
    assert!(recovery_steps.contains("lease_expires_at_utc"));
    assert!(recovery_steps.contains("CX-MM-009"));
    assert!(recovery_steps.contains(
        "internal_diagnostics is WIRED through the native producer and Problems projection"
    ));
    assert!(recovery_steps.contains(
        "Palmistry is WIRED through the authenticated watcher and survivor recovery importer"
    ));
}

/// WP-1 MT-008: Dexterity lane diagnostics must be discoverable from the
/// in-product UserManual in the same implementation change.
#[tokio::test]
async fn model_lane_diagnostics_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_diagnostics_manual");
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-diagnostics",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane diagnostics manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity Lane Diagnostics",
        "native_swarm_lane_diagnostics",
        "ModelLaneStore::diagnostics_projection",
        "ModelLaneStore::latest_diagnostics_projection",
        "GET /swarm/model-lanes/diagnostics/latest",
        "GET /swarm/model-lanes/diagnostics/{run_id}",
        // Code truth (top_menu_bar.rs): "Open Lane Diagnostics" lives under the
        // dedicated MODELS menu-bar dropdown (commit abd9c256, 2026-07-24), and
        // the seed page already states "MODELS > Open Lane Diagnostics". This
        // assertion was authored 2026-07-01 when the item was under RUN and was
        // not synced when the menu moved; keep the manual truthful to code.
        "MODELS > Open Lane Diagnostics",
        "swarmdiagnostics.open",
        "settings.swarm-lane-diagnostics-default-open",
        "menu.models.swarm-lane-diagnostics",
        "menu.run.problems",
        "settings.model-runtime.open-problems",
        "swarm-lane-diagnostics.surface",
        "swarm-lane-diagnostics.filter.run",
        "swarm-lane-diagnostics.filter.lane",
        "swarm-lane-diagnostics.filter.message",
        "swarm-lane-diagnostics.error",
        "payload and promotion drilldowns",
        "EventLedger event IDs",
        "flight_recorder_correlation_id",
        "EventLedger-backed alias",
        "FlightRecorder correlation",
        "trace/span/link IDs",
        "CRDT",
        "Locus",
        "Loom",
        "FEMS",
        "ContextBundle",
        "memory pack refs",
        "ArtifactStore refs",
        "HBR-INT-009",
        "flight_recorder",
        "internal_diagnostics",
        "palmistry",
        "DEFERRED-with-reason",
        "Argus",
        "React",
        "TypeScript",
        "Tauri",
        "WebView",
        "swarm_lane_diagnostics_backend_projection_matches_eventledger",
        "swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture",
        "swarm_lane_diagnostics_argus_lists_filters_and_drills_down",
        "swarm_lane_diagnostics_argus_rejects_missing_author_id_and_count_mismatch",
        "run_menu_opens_swarm_lane_diagnostics",
        "typing_diagnostics_filters_to_swarm_lane_diagnostics_and_runs",
        "swarm_lane_diagnostics_setting_persists",
        "model_lane_diagnostics_user_manual_entry_is_current",
    ] {
        assert!(
            body.contains(required),
            "model-lane diagnostics UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let tool_ids: std::collections::BTreeSet<_> = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    assert!(tool_ids.contains("swarm_lane_diagnostics_runtime_proof"));

    let diagnostics_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "swarm_lane_diagnostics_runtime_proof")
        .expect("diagnostics tool entry");
    assert_eq!(diagnostics_tool["status"], "wired");
    assert!(
        diagnostics_tool
            .to_string()
            .contains("swarm_lane_diagnostics_rejects_flight_recorder_only_hbr_posture"),
        "diagnostics tool metadata must include the HBR negative exact proof"
    );
    assert!(diagnostics_tool["description"]
        .as_str()
        .unwrap()
        .contains("native pane"));
    for required in [
        "native_swarm_lane_diagnostics projection",
        "GET /swarm/model-lanes/diagnostics/latest",
        "GET /swarm/model-lanes/diagnostics/{run_id}",
        "menu.models.swarm-lane-diagnostics",
        "swarm-lane-diagnostics.surface",
        "payload and promotion drilldowns",
        "EventLedger event IDs",
        "EventLedger-backed FlightRecorder correlation IDs and aliases",
        "FlightRecorder correlation IDs",
        "CRDT refs",
        "Locus/Loom/FEMS refs",
        "HBR-INT-009 tiers",
        "MT runtime status refs",
        "schema_id mismatch",
        "projection validation rejects missing author IDs",
        "missing internal_diagnostics/Palmistry tiers",
        "missing HBR tier state",
        "deferred tiers without follow_up_ref",
    ] {
        assert!(
            diagnostics_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "diagnostics tool expected_output must mention {required}"
        );
    }
    let schema_fields: std::collections::BTreeSet<_> = diagnostics_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect();
    for required in [
        "ModelLaneDiagnosticsProjection",
        "SwarmLaneDiagnosticsProjection",
        "SwarmLaneDiagnosticsPaneFactory",
        "SwarmLaneDiagnosticsClient",
        "ModelLaneStore::diagnostics_projection",
        "ModelLaneStore::latest_diagnostics_projection",
        "native_swarm_lane_diagnostics",
        "swarmdiagnostics.open",
        "settings.swarm-lane-diagnostics-default-open",
        "kernel_event_ledger",
        "model_lane_diagnostic_tier_statuses",
        "model_lane_mt_runtime_statuses",
        "FlightRecorder",
        "internal_diagnostics",
        "Palmistry",
        "Locus",
        "Loom",
        "FEMS",
    ] {
        assert!(
            schema_fields.contains(required),
            "diagnostics tool schema_fields must include {required}"
        );
    }
    let common_errors = diagnostics_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(common_errors.contains("projection_contract_mismatch"));
    assert!(common_errors.contains("lane_message_count_mismatch"));
    assert!(common_errors.contains("missing_stable_author_id"));
    assert!(common_errors.contains("missing_payload_ref"));
    assert!(common_errors.contains("missing_eventledger_evidence"));
    assert!(common_errors.contains("missing_flightrecorder_correlation"));
    assert!(common_errors.contains("missing_hbr_int_009_tier"));
    assert!(common_errors.contains("schema_id_mismatch"));
    assert!(common_errors.contains("missing_hbr_tier_state"));
    assert!(common_errors.contains("missing_deferred_follow_up_ref"));

    let recovery_steps = diagnostics_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("GET /swarm/model-lanes/diagnostics/{run_id}"));
    assert!(recovery_steps.contains("ModelLaneStore::replay_run"));
    assert!(recovery_steps.contains("Argus inspection"));
    assert!(recovery_steps.contains("EventLedger or FlightRecorder refs"));
    assert!(recovery_steps.contains("DEFERRED-with-reason"));
}

/// WP-1 MT-010: Dexterity ModelLane backend navigation must be discoverable
/// from the in-product UserManual and tied to exact Rust API proof commands.
#[tokio::test]
async fn model_lane_navigation_user_manual_entries_are_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_navigation_manual");
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-navigation",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane navigation manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);

    for required in [
        "Dexterity ModelLane Backend Navigation",
        "hsk.model_lane_navigation@1",
        "ModelLaneNavigationProjection",
        "ModelLaneStore::navigation_by_run",
        "ModelLaneStore::navigation_by_lane",
        "ModelLaneStore::navigation_by_message",
        "ModelLaneStore::navigation_by_artifact_or_context",
        "ModelLaneStore::navigation_by_trace",
        "ModelLaneStore::navigation_by_diagnostics",
        "ModelLaneStore::navigation_by_recovery",
        "ModelLaneStore::navigation_by_lookup",
        "GET /swarm/model-lanes/navigation/runs/{run_id}",
        "GET /swarm/model-lanes/navigation/lanes/{lane_id}",
        "GET /swarm/model-lanes/navigation/messages/{message_id}",
        "GET /swarm/model-lanes/navigation/artifacts",
        "GET /swarm/model-lanes/navigation/traces/{trace_id}",
        "GET /swarm/model-lanes/navigation/diagnostics/{run_id}",
        "GET /swarm/model-lanes/navigation/recovery/{run_id}",
        "GET /swarm/model-lanes/navigation/lookup",
        "model_session_id",
        "session_id",
        "wp_id",
        "mt_id",
        "task_board_id",
        "artifact_ref",
        "context_bundle_id",
        "Locus",
        "Loom",
        "FEMS",
        "MemoryPack",
        "EventLedger event IDs/sequences",
        "trace_id",
        "span_id",
        "error-code selectors",
        "PostgreSQL",
        "kernel_event_ledger",
        "Flight Recorder",
        "Palmistry",
        "model_lane_navigation_routes_return_run_lane_message_artifact_trace_and_recovery",
        "model_lane_navigation_user_manual_registry_rows_match_runtime_routes",
        "model_lane_navigation_user_manual_entries_are_current",
    ] {
        assert!(
            body.contains(required),
            "model-lane navigation UserManual page must mention {required}"
        );
    }

    let registry_ids: std::collections::BTreeSet<_> = wp009_surface_registry()
        .iter()
        .filter(|surface| surface.group == SurfaceGroup::ModelLaneNavigation)
        .map(|surface| surface.surface_id)
        .collect();
    for required in [
        "model_lane.navigation.run",
        "model_lane.navigation.lane",
        "model_lane.navigation.message",
        "model_lane.navigation.artifact_context",
        "model_lane.navigation.trace_span",
        "model_lane.navigation.diagnostic_tier",
        "model_lane.navigation.recovery",
        "model_lane.navigation.lookup",
    ] {
        assert!(
            registry_ids.contains(required),
            "navigation registry must include {required}"
        );
    }

    let wp009_tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp009_surface&limit=500",
            fx.base
        ))
        .send()
        .await
        .expect("list wp009 tools")
        .json()
        .await
        .expect("tools json");
    let wp009_tool_ids: std::collections::BTreeSet<_> = wp009_tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .filter_map(|tool| tool["tool_id"].as_str())
        .collect();
    for required in registry_ids {
        assert!(
            wp009_tool_ids.contains(required),
            "wp009 tool entries must include navigation surface {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let navigation_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "model_lane_navigation_api_tests")
        .expect("model lane navigation tool entry");
    assert_eq!(navigation_tool["status"], "wired");
    for required in [
        "ModelLaneNavigationProjection",
        "ModelLaneNavigationLookup",
        "ModelLaneStore::navigation_by_lookup",
        "GET /swarm/model-lanes/navigation/lookup",
        "model_lane.navigation.lookup",
        "event_ledger_event_id",
        "event_ledger_seq",
        "trace_id",
        "span_id",
        "error_code",
        "Flight Recorder",
        "EventLedger",
        "internal_diagnostics",
        "Palmistry",
        "Locus",
        "Loom",
        "FEMS",
        "ContextBundle",
        "MemoryPack",
    ] {
        assert!(
            navigation_tool.to_string().contains(required),
            "navigation tool entry must mention {required}"
        );
    }
}

/// WP-1 MT-009: the mixed-lane validation harness must be discoverable from
/// the in-product UserManual and tied to exact Rust proof commands.
#[tokio::test]
async fn model_lane_validation_harness_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "model_lane_validation_harness_manual");
    let response = fx
        .http
        .get(format!(
            "{}/usermanual/pages/model-lane-validation-harness",
            fx.base
        ))
        .send()
        .await
        .expect("read ModelLane validation harness manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    let body = page.to_string();
    assert_internal_diagnostics_and_palmistry_wired(&body);
    let page_body_md = page["sections"]
        .as_array()
        .expect("manual page sections")
        .iter()
        .filter_map(|section| section["body_md"].as_str())
        .collect::<Vec<_>>()
        .join("\n");

    for required in [
        "Dexterity Mixed-Lane Validation Harness",
        "mixed local/cloud/subagent ModelLaneRun",
        "native_swarm_lane_diagnostics",
        "hsk.user_manual_behavior_coverage@1",
        "model_lane_behavior_coverage_matrix",
        "verify_model_lane_behavior_coverage",
        "FlightRecorder",
        "internal_diagnostics",
        "Palmistry",
        "mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects",
        "mixed_model_lane_negative_guards_fail_closed",
        "mt009_yjs_atomic_cross_connection_race_keeps_eventledger_and_crdt_receipts_in_lockstep",
        "ac9_bounded_retry_exhaustion_fails_after_three_durable_attempts",
        "ac9_cancel_terminalizes_parallel_siblings_and_live_sessions",
        "ac9_crash_after_persisted_spawn_intent_recovers_with_new_fence_and_compensation",
        "ac9_cancel_and_peer_failure_propagate_into_blocked_factory_create",
        "operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity",
        "mixed_model_lane_run_is_inspectable_through_argus",
        "mixed_model_lane_behaviors_have_manual_coverage",
        "Ordinary generated routing text is an advisory Proposal",
        "remain null for all six routing policies",
        "canonical PostgreSQL Yjs bytes",
        "post-update state vector",
        "inventing a `crdt-*://` URI",
    ] {
        assert!(
            body.contains(required),
            "model-lane validation harness UserManual page must mention {required}"
        );
    }

    let tools: Value = fx
        .http
        .get(format!(
            "{}/usermanual/tools?origin=wp1_model_lane",
            fx.base
        ))
        .send()
        .await
        .expect("list model-lane manual tools")
        .json()
        .await
        .expect("tools json");
    let validation_tool = tools["tools"]
        .as_array()
        .expect("tools array")
        .iter()
        .find(|tool| tool["tool_id"] == "mixed_model_lane_integration_pg_tests")
        .expect("mixed model-lane validation tool entry");
    assert_eq!(validation_tool["status"], "wired");

    for exact_test in [
        "mixed_local_cloud_subagent_run_persists_restarts_replays_and_projects",
        "mixed_model_lane_negative_guards_fail_closed",
        "mixed_concurrent_model_and_operator_lanes_converge_on_shared_crdt_key",
        "mt009_midstream_cancellation_preserves_prefix_and_rejects_late_messages",
        "mt009_real_postgres_yjs_updates_compaction_receipts_and_lane_state_converge",
        "mt009_yjs_atomic_cross_connection_race_keeps_eventledger_and_crdt_receipts_in_lockstep",
        "ac9_bounded_retry_exhaustion_fails_after_three_durable_attempts",
        "ac9_cancel_terminalizes_parallel_siblings_and_live_sessions",
        "ac9_crash_after_persisted_spawn_intent_recovers_with_new_fence_and_compensation",
        "ac9_cancel_and_peer_failure_propagate_into_blocked_factory_create",
        "operator_chat_launch_coordinator_cancellation_preserves_prefix_and_rejects_late_activity",
        "coordinator_cancellation_fence_rejects_generation_during_terminal_pg_write",
        "coordinator_cancellation_fence_retries_after_terminal_pg_failure",
        "mixed_model_lane_run_is_inspectable_through_argus",
        "model_lane_validation_harness_user_manual_entry_is_current",
        "mixed_model_lane_behaviors_have_manual_coverage",
    ] {
        assert!(
            page_body_md.contains(exact_test),
            "manual proof commands must include exact MT-009 test {exact_test}"
        );
        assert!(
            validation_tool.to_string().contains(exact_test),
            "validation tool metadata must include exact MT-009 test {exact_test}"
        );
    }

    for required in [
        "ProjectionPlan/ConsentReceipt",
        "CRDT",
        "recovery checkpoints",
        "native AccessKit Argus harness",
    ] {
        assert!(
            validation_tool["expected_input"]
                .as_str()
                .unwrap()
                .contains(required),
            "validation tool expected_input must mention {required}"
        );
    }
    for required in [
        "A replayable mixed ModelLaneRun",
        "backend lane/message counts matching native diagnostics rows",
        "Rust coverage matrix/contract entries",
        "FlightRecorder/internal_diagnostics/Palmistry",
        "stable-lane restart_generation advancement",
        "exactly-once teardown",
    ] {
        assert!(
            validation_tool["expected_output"]
                .as_str()
                .unwrap()
                .contains(required),
            "validation tool expected_output must mention {required}"
        );
    }
    let schema_fields = validation_tool["schema_fields"]
        .as_array()
        .expect("schema fields")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for required in [
        "hsk.user_manual_behavior_coverage@1",
        "ModelLaneStore::replay_run",
        "ModelLaneStore::recover_run_after_restart",
        "ModelLaneStore::diagnostics_projection",
        "model_lane_behavior_coverage_matrix",
        "verify_model_lane_behavior_coverage",
        "native_swarm_lane_diagnostics",
        "restart_generation",
        "CRDT",
        "Locus",
        "Loom",
        "FEMS",
    ] {
        assert!(
            schema_fields.contains(required),
            "validation tool schema_fields must include {required}"
        );
    }
    let common_errors = validation_tool["common_errors"]
        .as_array()
        .expect("common errors")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for required in [
        "direct_endpoint_bypass",
        "missing_cloud_consent",
        "missing_payload_authority",
        "stale_crdt_base",
        "replay_order_gap",
        "argus_count_mismatch",
        "missing_manual_coverage",
        "FlightRecorder-only",
        "model_lane_restart_generation_conflict",
        "orphaned_session_teardown",
    ] {
        assert!(
            common_errors.contains(required),
            "validation tool common_errors must mention {required}"
        );
    }
    let recovery_steps = validation_tool["recovery_steps"]
        .as_array()
        .expect("recovery steps")
        .iter()
        .filter_map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(recovery_steps.contains("ModelLaneStore::replay_run"));
    assert!(recovery_steps.contains("ModelLaneStore::recover_run_after_restart"));
    assert!(recovery_steps.contains("claim.attempt N"));
    assert!(recovery_steps.contains("generation-scoped model_lane/model_lane_terminal"));
    assert!(recovery_steps.contains("Repair missing payloads"));
    assert!(recovery_steps.contains("Reject stale CRDT bases"));
    assert!(recovery_steps.contains("Repair UserManual gaps"));
    assert!(
        page_body_md.contains("--target-dir ..\\Handshake_Artifacts\\handshake-cargo-target"),
        "validation harness UserManual page must embed external target-dir Cargo commands"
    );
    assert!(
        validation_tool
            .to_string()
            .contains("--target-dir ..\\\\Handshake_Artifacts\\\\handshake-cargo-target"),
        "validation harness tool metadata must embed external target-dir Cargo commands"
    );
}

/// WP-1 MT-023: the built-in manual must describe the runtime-proven
/// worktree-scoped Cloud Hypervisor model-lane path, shipped registry and
/// detached reclaim without inventing a public Tier3/WarmVm request selector.
#[tokio::test]
async fn microvm_worktree_model_lane_user_manual_entry_is_current() {
    let fx = skip_if_no_pg!(fixture().await, "microvm_worktree_model_lane_manual");
    let response = fx
        .http
        .get(format!("{}/usermanual/pages/microvm-sandbox-kvm", fx.base))
        .send()
        .await
        .expect("read microVM worktree-lane manual page");
    assert_eq!(response.status(), 200);
    let page: Value = response.json().await.expect("manual page json");
    assert_eq!(page["page"]["manual_version"], USER_MANUAL_VERSION);
    let body = page.to_string();

    for required in [
        "ModelLaneStore",
        "SwarmCoordinator",
        "ProductionModelSessionFactory",
        "WorktreeVmRegistry",
        "CloudHypervisorAdapter",
        "build_production_swarm_coordinator_with_sandbox_registry",
        "production_process_sandbox_registry_async",
        "ProcessReclaimRuntime",
        "routes_with_process_reclaim_runtime",
        "process_runtime.sandbox_registry()",
        "IsolationTier::Tier3Microvm",
        "with_warm_vm_execution()",
        "/worktree/AGENTS.md",
        "owner_account_id",
        "actor_principal_id",
        "authenticated_session_id",
        "access_space_id",
        "workspace_id",
        "worktree_id",
        "cross-account",
        "ProcessOwnershipLedger",
        "worktree_model_lane_live_pg_tests",
        "mt023_real_worktree_model_lane_runs_inside_microvm",
        "--ignored",
        "fresh adapter",
        "WorktreeVmRegistry::teardown_worktree_vm",
        "Public `/operator-chat/launch`",
        "WP-KERNEL-012 is unshipped",
    ] {
        assert!(
            body.contains(required),
            "microVM worktree-lane UserManual page must mention {required}"
        );
    }
    for stale in [
        "NOT YET WIRED",
        "The proven run was driven directly against the VMM",
        "compiled-in default VMM and kernel paths are machine-local",
        "does not yet register that adapter",
        "full host-process restart is not proven",
    ] {
        assert!(
            !body.contains(stale),
            "microVM worktree-lane UserManual page must not retain stale claim {stale}"
        );
    }
}

/// MT-201: page linking — outbound page links and inbound backlinks resolve.
#[tokio::test]
async fn mt201_page_links_resolve() {
    let fx = skip_if_no_pg!(fixture().await, "mt201_links");
    let links: Value = fx
        .http
        .get(format!("{}/usermanual/pages/manual-toc/links", fx.base))
        .send()
        .await
        .expect("toc links")
        .json()
        .await
        .expect("links json");
    let outbound: Vec<&str> = links["outbound"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(outbound.contains(&"quickstart-index"));
    assert!(outbound.contains(&"state-recovery-guide"));

    let recovery: Value = fx
        .http
        .get(format!(
            "{}/usermanual/pages/state-recovery-guide/links",
            fx.base
        ))
        .send()
        .await
        .expect("recovery links")
        .json()
        .await
        .expect("recovery json");
    let inbound: Vec<&str> = recovery["inbound"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        inbound.contains(&"manual-toc"),
        "TOC links every page; inbound must show it (got {inbound:?})"
    );
}

/// MT-199: every contract area returns a bundled quickstart with linked
/// pages inlined; an unknown area is a typed 404.
#[tokio::test]
async fn mt199_quickstart_bundles_resolve_all_areas() {
    let fx = skip_if_no_pg!(fixture().await, "mt199_quickstarts");
    for area in QUICKSTART_AREAS {
        let bundle: Value = fx
            .http
            .get(format!("{}/usermanual/quickstarts/{area}", fx.base))
            .send()
            .await
            .expect("quickstart")
            .json()
            .await
            .expect("quickstart json");
        assert_eq!(bundle["area"], *area);
        assert!(
            !bundle["quickstart"]["sections"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{area} quickstart has sections"
        );
        assert!(
            !bundle["linked_pages"].as_array().unwrap().is_empty(),
            "{area} quickstart inlines its linked pages"
        );
        let receipt = bundle["bootstrap_receipt_event_id"].as_str().unwrap();
        assert!(receipt_exists(&fx.kpg, receipt).await);
    }
    let missing = fx
        .http
        .get(format!("{}/usermanual/quickstarts/zzz", fx.base))
        .send()
        .await
        .expect("unknown area");
    assert_eq!(missing.status(), 404);
}

/// MT-199 negative guard: quickstart bundles must fail closed when a seeded
/// `page_link` target is missing. Returning 200 with a silently omitted linked
/// page would hand a no-context model an incomplete bootstrap bundle.
#[tokio::test]
async fn mt199_quickstart_fails_when_linked_page_is_missing() {
    let fx = skip_if_no_pg!(fixture().await, "mt199_quickstart_missing_link");
    let mut conn = fx.kpg.raw_connection().await;
    let changed_anchor: String = sqlx::query_scalar(
        r#"
        WITH victim AS (
            SELECT a.anchor_id
            FROM user_manual_anchors a
            JOIN user_manual_pages p ON p.page_id = a.page_id
            WHERE p.slug = 'quickstart-index'
              AND a.anchor_kind = 'page_link'
            ORDER BY a.anchor_value
            LIMIT 1
        )
        UPDATE user_manual_anchors
        SET anchor_value = 'missing-linked-page-for-mt199'
        WHERE anchor_id = (SELECT anchor_id FROM victim)
        RETURNING anchor_id
        "#,
    )
    .fetch_one(&mut conn)
    .await
    .expect("tamper quickstart page_link");
    conn.close().await.ok();

    let response = fx
        .http
        .get(format!("{}/usermanual/quickstarts/index", fx.base))
        .send()
        .await
        .expect("quickstart with missing link");
    assert_eq!(
        response.status(),
        404,
        "quickstart with tampered anchor {changed_anchor} must fail closed"
    );
    let body: Value = response.json().await.expect("missing link error json");
    assert_eq!(body["error"], "not_found");
    assert!(
        body["detail"]
            .as_str()
            .unwrap_or_default()
            .contains("missing-linked-page-for-mt199"),
        "missing link error should name the unresolved page_link: {body:?}"
    );
}

/// MT-200: access points cover the five contract host surfaces and every
/// target slug resolves against the LIVE database.
#[tokio::test]
async fn mt200_access_points_resolve() {
    let fx = skip_if_no_pg!(fixture().await, "mt200_access_points");
    let payload: Value = fx
        .http
        .get(format!("{}/usermanual/access-points", fx.base))
        .send()
        .await
        .expect("access points")
        .json()
        .await
        .expect("access json");
    let rows = payload["access_points"].as_array().unwrap();
    assert!(rows.len() >= 11);
    let mut hosts = std::collections::BTreeSet::new();
    for row in rows {
        assert_eq!(
            row["target_resolves"], true,
            "access point {} targets a missing page {}",
            row["access_point_id"], row["target_page_slug"]
        );
        let stable_element_id = row["stable_element_id"].as_str().unwrap();
        assert!(
            stable_element_id.starts_with("hs-usermanual-")
                || stable_element_id == "menu.run.user-manual"
                || stable_element_id == "menu.help.user-manual"
                || stable_element_id == "user-manual.surface"
        );
        hosts.insert(row["host_surface"].as_str().unwrap().to_string());
    }
    for host in [
        "editor",
        "notes_loom",
        "retrieval_debug",
        "diagnostics",
        "command_palette",
        "native_top_menu",
        "native_user_manual",
    ] {
        assert!(hosts.contains(host), "missing host surface {host}");
    }
}

/// MT-203: the legacy bridge answers with the canonical mapping AND a
/// persisted compatibility receipt (spec 10.15.8 bridge law).
#[tokio::test]
async fn mt203_legacy_bridge_route_maps_and_emits_compat_receipt() {
    let fx = skip_if_no_pg!(fixture().await, "mt203_legacy_bridge");
    let bridge: Value = fx
        .http
        .get(format!("{}/usermanual/legacy/model-manual", fx.base))
        .send()
        .await
        .expect("legacy bridge")
        .json()
        .await
        .expect("bridge json");
    assert_eq!(bridge["deprecated"], true);
    assert!(!bridge["canonical"]["pages"].as_array().unwrap().is_empty());
    assert_eq!(bridge["canonical"]["route_namespace"], "/usermanual");
    let receipt = bridge["compatibility_receipt_event_id"].as_str().unwrap();
    assert!(
        receipt_exists(&fx.kpg, receipt).await,
        "compatibility receipt must be persisted (spec 10.15.8)"
    );

    let aliases: Value = fx
        .http
        .get(format!("{}/usermanual/legacy/aliases", fx.base))
        .send()
        .await
        .expect("aliases")
        .json()
        .await
        .expect("aliases json");
    assert!(aliases["count"].as_u64().unwrap() >= 6);

    let plan: Value = fx
        .http
        .get(format!("{}/usermanual/migration-plan", fx.base))
        .send()
        .await
        .expect("plan")
        .json()
        .await
        .expect("plan json");
    assert_eq!(plan["canonical_term"], "UserManual");
    assert!(plan["rows"].as_array().unwrap().len() >= 13);
}

/// MT-204: freshness is `current` after seed; a tampered page flips to
/// stale_content; restoring heals. The check itself is receipted.
#[tokio::test]
async fn mt204_freshness_current_then_stale_fixture() {
    let fx = skip_if_no_pg!(fixture().await, "mt204_freshness");
    let fresh: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("freshness")
        .json()
        .await
        .expect("freshness json");
    assert_eq!(
        fresh["report"]["fresh"],
        true,
        "seeded manual must be fresh: {:?}",
        fresh["report"]["verdicts"].as_array().map(|v| v
            .iter()
            .filter(|x| x["kind"] != "current")
            .collect::<Vec<_>>())
    );
    let receipt = fresh["receipt_event_id"].as_str().unwrap();
    assert!(receipt_exists(&fx.kpg, receipt).await);

    // Stale fixture (MT-208 family): tamper one stored page hash.
    let previous = tamper_page_content_hash(&fx.kpg.db, "core-workflows")
        .await
        .expect("tamper");
    let stale: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("stale freshness")
        .json()
        .await
        .expect("stale json");
    assert_eq!(stale["report"]["fresh"], false);
    assert!(
        stale["report"]["verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|v| { v["kind"] == "stale_content" && v["subject"] == "core-workflows" }),
        "tampered page must yield stale_content"
    );

    restore_page_content_hash(&fx.kpg.db, "core-workflows", &previous)
        .await
        .expect("restore");
    let healed: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("healed freshness")
        .json()
        .await
        .expect("healed json");
    assert_eq!(healed["report"]["fresh"], true);
}

/// MT-205: the HTML projection carries stable selectors, ordered sections,
/// escaped content, and resolvable navigation; every stored page is
/// reachable from the TOC.
#[tokio::test]
async fn mt205_projection_renders_readable_navigable_html() {
    let fx = skip_if_no_pg!(fixture().await, "mt205_projection");
    let projection: Value = fx
        .http
        .get(format!(
            "{}/usermanual/pages/manual-toc/projection?format=html",
            fx.base
        ))
        .send()
        .await
        .expect("projection")
        .json()
        .await
        .expect("projection json");
    let html = projection["rendered"].as_str().unwrap();
    assert!(html.contains("data-hs-manual-page=\"manual-toc\""));
    assert!(html.contains("data-hs-manual-section="));
    assert!(html.contains("data-hs-manual-link=\"quickstart-index\""));
    assert!(html.contains("data-hs-href=\"/usermanual/pages/quickstart-index\""));
    assert!(
        !html.contains("<script>"),
        "projection must never emit live script"
    );

    let markdown: Value = fx
        .http
        .get(format!(
            "{}/usermanual/pages/manual-toc/projection?format=markdown",
            fx.base
        ))
        .send()
        .await
        .expect("md projection")
        .json()
        .await
        .expect("md json");
    assert!(markdown["rendered"]
        .as_str()
        .unwrap()
        .contains("<topic id=\"manual-toc-0\""));

    let bad = fx
        .http
        .get(format!(
            "{}/usermanual/pages/manual-toc/projection?format=pdf",
            fx.base
        ))
        .send()
        .await
        .expect("bad format");
    assert_eq!(bad.status(), 400);

    // Visual navigation law: no stored page is orphaned from the TOC.
    let orphans = unreachable_pages(&fx.kpg.db).await.expect("nav audit");
    assert!(orphans.is_empty(), "orphan manual pages: {orphans:?}");
}

/// Metadata-only drift in the same manual version must not hide behind clean
/// page/tool/feature rows. Governed resync rewrites the version authority row.
#[tokio::test]
async fn metadata_only_same_version_drift_is_stale_and_resync_heals_it() {
    let fx = skip_if_no_pg!(fixture().await, "metadata_only_same_version_drift");
    let mut conn = fx.kpg.raw_connection().await;
    let original: (String, i32, i32, i32) = sqlx::query_as(
        r#"
        SELECT seed_content_hash, page_count, tool_count, feature_count
        FROM user_manual_versions
        WHERE manual_version = $1
        "#,
    )
    .bind(USER_MANUAL_VERSION)
    .fetch_one(&mut conn)
    .await
    .expect("read original UserManual version metadata");

    sqlx::query(
        r#"
        UPDATE user_manual_versions
        SET seed_content_hash = '0000000000000000000000000000000000000000000000000000000000000000'
        WHERE manual_version = $1
        "#,
    )
    .bind(USER_MANUAL_VERSION)
    .execute(&mut conn)
    .await
    .expect("tamper only UserManual version hash metadata");

    let stale: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("metadata freshness request")
        .json()
        .await
        .expect("metadata freshness json");
    assert_eq!(stale["report"]["fresh"], false);
    assert!(
        stale["report"]["verdicts"]
            .as_array()
            .expect("freshness verdicts")
            .iter()
            .any(|verdict| verdict["subject"] == USER_MANUAL_VERSION),
        "hash-only metadata drift must identify the active UserManual version"
    );

    let resync = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "local_model")
        .send()
        .await
        .expect("governed metadata resync");
    assert_eq!(resync.status(), 200);

    let healed: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("healed metadata freshness request")
        .json()
        .await
        .expect("healed metadata freshness json");
    assert_eq!(healed["report"]["fresh"], true);

    sqlx::query(
        r#"
        UPDATE user_manual_versions
        SET page_count = page_count + 1,
            tool_count = tool_count + 1,
            feature_count = feature_count + 1
        WHERE manual_version = $1
        "#,
    )
    .bind(USER_MANUAL_VERSION)
    .execute(&mut conn)
    .await
    .expect("tamper only UserManual version count metadata");

    let count_stale: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("count metadata freshness request")
        .json()
        .await
        .expect("count metadata freshness json");
    assert_eq!(count_stale["report"]["fresh"], false);
    assert!(
        count_stale["report"]["verdicts"]
            .as_array()
            .expect("count freshness verdicts")
            .iter()
            .any(|verdict| verdict["subject"] == USER_MANUAL_VERSION),
        "count-only metadata drift must identify the active UserManual version"
    );

    let count_resync = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "local_model")
        .send()
        .await
        .expect("governed count metadata resync");
    assert_eq!(count_resync.status(), 200);

    let count_healed: Value = fx
        .http
        .get(format!("{}/usermanual/freshness", fx.base))
        .send()
        .await
        .expect("healed count metadata freshness request")
        .json()
        .await
        .expect("healed count metadata freshness json");
    assert_eq!(count_healed["report"]["fresh"], true);

    let repaired: (String, i32, i32, i32) = sqlx::query_as(
        r#"
        SELECT seed_content_hash, page_count, tool_count, feature_count
        FROM user_manual_versions
        WHERE manual_version = $1
        "#,
    )
    .bind(USER_MANUAL_VERSION)
    .fetch_one(&mut conn)
    .await
    .expect("read repaired UserManual version metadata");
    assert_eq!(repaired, original);
}

/// Resync permission gate: unauthenticated/cloud_model/validator are DENIED
/// with stable reasons; unknown tokens are 400; local_model succeeds.
#[tokio::test]
async fn mt201_resync_permission_gate_fails_closed() {
    let fx = skip_if_no_pg!(fixture().await, "mt201_resync_gate");

    let anonymous = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .send()
        .await
        .expect("anonymous resync");
    assert_eq!(anonymous.status(), 403);
    let body: Value = anonymous.json().await.expect("403 json");
    assert_eq!(body["reason"], "unauthenticated_resync_denied");

    let cloud = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "cloud_model")
        .send()
        .await
        .expect("cloud resync");
    assert_eq!(cloud.status(), 403);
    let body: Value = cloud.json().await.expect("403 json");
    assert_eq!(body["reason"], "cloud_model_resync_denied");

    let unknown = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "root")
        .send()
        .await
        .expect("unknown kind");
    assert_eq!(
        unknown.status(),
        400,
        "unknown tokens are rejected, never coerced"
    );

    let allowed = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "local_model")
        .send()
        .await
        .expect("local_model resync");
    assert_eq!(allowed.status(), 200);
    let report: Value = allowed.json().await.expect("resync json");
    assert_eq!(
        report["resync"]["pages_changed"], 0,
        "already-seeded resync is a no-op"
    );
}

/// THE doc-vs-runtime keystone: every surface the manual declares is probed
/// against the REAL full product router. A documented route the router does
/// not mount (router-level 404: empty body) or a wrong documented method
/// (405) fails the suite — the manual cannot describe surfaces the product
/// does not serve (spec 10.15.8: stale docs are a build defect).
#[tokio::test]
async fn mtdoc_every_registry_surface_exists_on_the_real_router() {
    let kpg = skip_if_no_pg!(
        knowledge_pg_support::knowledge_pg().await,
        "mtdoc_router_probe"
    );
    ensure_seeded(&kpg.db).await.expect("seed");
    let state = app_state_for(&kpg.schema_url).await;
    let (base, _server) = start_server(api::routes(state)).await;
    let http = reqwest::Client::new();

    for surface in wp009_surface_registry() {
        let path = probe_path(surface.route);
        let url = format!("{base}{path}");
        let request = match surface.method {
            "GET" => http.get(&url),
            "POST" => http.post(&url),
            "PUT" => http.put(&url),
            "DELETE" => http.delete(&url),
            "PATCH" => http.patch(&url),
            other => panic!("unsupported method {other}"),
        };
        let response = request.send().await.unwrap_or_else(|err| {
            panic!(
                "probe {} {} failed to send: {err}",
                surface.method, surface.route
            )
        });
        let status = response.status();
        assert_ne!(
            status,
            reqwest::StatusCode::METHOD_NOT_ALLOWED,
            "manual documents {} {} but the router answers 405 — wrong method documented",
            surface.method,
            surface.route
        );
        if status == reqwest::StatusCode::NOT_FOUND {
            let body = response.text().await.unwrap_or_default();
            assert!(
                !body.trim().is_empty(),
                "manual documents {} {} but the router has NO such route (bare 404)",
                surface.method,
                surface.route
            );
        }
    }
}
