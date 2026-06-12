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
use handshake_core::user_manual::registry::{probe_path, wp009_surface_registry};
use handshake_core::user_manual::seed::{ensure_seeded, QUICKSTART_AREAS};
use handshake_core::user_manual::USER_MANUAL_VERSION;
use knowledge_pg_support::KnowledgePg;
use serde_json::Value;
use sqlx::Connection;
use user_manual_support::{app_state_for, start_server};

struct ApiFixture {
    kpg: KnowledgePg,
    base: String,
    _server: tokio::task::JoinHandle<()>,
    http: reqwest::Client,
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
    let receipt = page["bootstrap_receipt_event_id"].as_str().expect("receipt id");
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
    assert!(found["count"].as_u64().unwrap() > 0, "seeded corpus documents backlinks");

    let empty = fx
        .http
        .get(format!("{}/usermanual/search", fx.base))
        .send()
        .await
        .expect("empty search");
    assert_eq!(empty.status(), 400);
    let body: Value = empty.json().await.expect("400 json");
    assert_eq!(body["error"], "bad_request");
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
            .get(format!("{}/usermanual/tools/{}", fx.base, surface.surface_id))
            .send()
            .await
            .expect("read code-nav tool")
            .json()
            .await
            .expect("tool json");
        assert_eq!(tool["tool"]["http_route"], surface.route);
        assert_eq!(tool["tool"]["http_method"], surface.method);
        assert!(!tool["tool"]["common_errors"].as_array().unwrap().is_empty());
        assert!(!tool["tool"]["recovery_steps"].as_array().unwrap().is_empty());
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
    assert!(legacy["count"].as_u64().unwrap() > 50, "legacy manifest imported");
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
            !bundle["quickstart"]["sections"].as_array().unwrap().is_empty(),
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
    assert!(rows.len() >= 8);
    let mut hosts = std::collections::BTreeSet::new();
    for row in rows {
        assert_eq!(
            row["target_resolves"], true,
            "access point {} targets a missing page {}",
            row["access_point_id"], row["target_page_slug"]
        );
        assert!(row["stable_element_id"].as_str().unwrap().starts_with("hs-usermanual-"));
        hosts.insert(row["host_surface"].as_str().unwrap().to_string());
    }
    for host in ["editor", "notes_loom", "retrieval_debug", "diagnostics", "command_palette"] {
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
    assert_eq!(fresh["report"]["fresh"], true, "seeded manual must be fresh: {:?}",
        fresh["report"]["verdicts"].as_array().map(|v| v.iter().filter(|x| x["kind"] != "current").collect::<Vec<_>>()));
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
        stale["report"]["verdicts"].as_array().unwrap().iter().any(|v| {
            v["kind"] == "stale_content" && v["subject"] == "core-workflows"
        }),
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
    assert!(!html.contains("<script>"), "projection must never emit live script");

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
    assert!(markdown["rendered"].as_str().unwrap().contains("<topic id=\"manual-toc-0\""));

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
    assert_eq!(unknown.status(), 400, "unknown tokens are rejected, never coerced");

    let allowed = fx
        .http
        .post(format!("{}/usermanual/resync", fx.base))
        .header("x-hsk-actor-kind", "local_model")
        .send()
        .await
        .expect("local_model resync");
    assert_eq!(allowed.status(), 200);
    let report: Value = allowed.json().await.expect("resync json");
    assert_eq!(report["resync"]["pages_changed"], 0, "already-seeded resync is a no-op");
}

/// THE doc-vs-runtime keystone: every surface the manual declares is probed
/// against the REAL full product router. A documented route the router does
/// not mount (router-level 404: empty body) or a wrong documented method
/// (405) fails the suite — the manual cannot describe surfaces the product
/// does not serve (spec 10.15.8: stale docs are a build defect).
#[tokio::test]
async fn mtdoc_every_registry_surface_exists_on_the_real_router() {
    let kpg = skip_if_no_pg!(knowledge_pg_support::knowledge_pg().await, "mtdoc_router_probe");
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
            panic!("probe {} {} failed to send: {err}", surface.method, surface.route)
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
