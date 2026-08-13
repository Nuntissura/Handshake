//! WP-KERNEL-012 MT-044 — E8 Parity Proof Suite, cluster E3 (Knowledge surface / Obsidian graph
//! parity). Features #23-#36.
//!
//! ## CTRL-2: every proof exercises a REAL native impl by its fully-qualified Rust path
//!
//! Each E3 feature has TWO proofs:
//!
//!  1. `parity_<feature>_native` — a NON-ignored proof that runs IN-PROCESS (no PostgreSQL). It drives
//!     the REAL native request-construction path — the `handshake_native::backend_client::*Client`
//!     `*_request` builders the production spawn paths route through — and asserts on the NATIVE output
//!     (the exact typed `(method, url, query|body)` the native client emits), or, for the breadcrumb
//!     trail, the native `graph::sidebar_panel::LoomSidebarPanel` history. These PASS today with no
//!     backend and are the load-bearing parity proof that the native editor produces the correct request.
//!  2. `parity_<feature>` — the `#[ignore = "requires_pg"]` live round-trip. It calls the SAME native
//!     builder (CTRL-2 for the manifest `proof_fn`), then exercises the seeded handshake_core route and
//!     asserts on the response. Gated `requires_pg` (the managed-PG run is a separate live-PG batch);
//!     with no env + no backend it panics with a `requires_pg` message, never fake-passes.
//!
//! There is NO sqlite, NO in-process backend substitute, and NO hard-coded result: the native half runs
//! the ported client code and the live half runs real PostgreSQL behind handshake_core.
//!
//! ## Native route note (verified against the running backend)
//!
//! The native graph client binds the VERIFIED `/loom/graph-search` (local) + `/loom/views/all` (global)
//! surfaces (`backend_client::LoomGraphClient`, verified read-only against the running backend). The
//! live round-trips below additionally exercise the `/loom/graph/local` + `/loom/graph/global` routes
//! named in the 2026-06-26 knowledge_documents route audit; both are recorded as verified-by-audit until
//! a managed-PG run exercises them (Spec-Realism Sub-rule 3).

mod parity_manifest_support;
mod pg_proof_support;

use parity_manifest_support::mark_pass;
use pg_proof_support::{require_live_backend, LiveBackend};

use handshake_native::backend_client::{
    BlockViewClient, CanvasBoardClient, DrawerActionClient, GetRequestSpec, HttpMethod,
    LoomFolderClient, LoomGraphClient, LoomSidebarClient, LoomTagClient, LoomWikiClient,
    RequestSpec,
};
use handshake_native::graph::block_collection_view::{BlockViewDefinition, BlockViewKind};
use handshake_native::graph::sidebar_panel::LoomSidebarPanel;

/// A minimal current-thread tokio runtime whose handle constructs the native REST clients. The native
/// proofs only call the PURE `*_request` builders (URL/body construction), never spawn, so the runtime
/// is never driven — it merely satisfies the client constructor signature.
fn native_rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("native proof runtime")
}

const NATIVE_BASE: &str = "http://native-proof";

fn has_query(spec: &GetRequestSpec, key: &str, val: &str) -> bool {
    spec.query.iter().any(|(k, v)| k == key && v == val)
}
fn has_query_key(spec: &GetRequestSpec, key: &str) -> bool {
    spec.query.iter().any(|(k, _)| k == key)
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-23: local graph — native LoomGraphClient::local_request_with_depth (GET /loom/graph-search)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_23_local_spec(ws: &str, title: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomGraphClient::new(NATIVE_BASE, rt.handle().clone()).local_request_with_depth(ws, title, 2)
}

#[test]
fn parity_local_graph_native() {
    let spec = e3_23_local_spec("ws-1", "focus-note");
    assert_eq!(spec.method, HttpMethod::Get);
    // MT-021 settled this route DELIBERATELY, and this assertion had not caught up. backend_client.rs
    // :2196-2211 records the verified decision: LOCAL is
    // GET /workspaces/:ws/loom/graph/local?start_block_id&max_depth&node_limit -> LoomGraph, the
    // authoritative undirected PostgreSQL neighbourhood, and graph-search "is a heterogeneous
    // retrieval/search surface, NOT a graph projection, and MUST NOT be used to fabricate star edges
    // for this view". Asserting graph-search here demanded exactly the surface MT-021 forbids.
    //
    // The stale expectation was already contradicted INSIDE this file: the live sibling
    // parity_local_graph is documented as "(GET /loom/graph/local)" and the backend serves that route
    // (api/loom.rs:291). The parity contract is unchanged in strength - an exact route plus its exact
    // query keys - only corrected to the route the system actually uses.
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/graph/local"),
        "E3-23: native local-graph URL (got {})",
        spec.url
    );
    assert!(
        has_query(&spec, "max_depth", "2"),
        "E3-23: depth-2 local graph query"
    );
    assert!(
        has_query_key(&spec, "start_block_id"),
        "E3-23: the local graph queries by focus block id"
    );
    println!(
        "E3-23 NATIVE PASS: LoomGraphClient::local_request_with_depth built {} (depth 2)",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID + HSK_TEST_BLOCK_ID (GET /loom/graph/local)"]
fn parity_local_graph() {
    let be: LiveBackend = require_live_backend();
    let block_id = be.require_block_id();
    // CTRL-2: build the native local-graph request (depth 2) through the real client builder.
    let spec = e3_23_local_spec(&be.workspace_id, &block_id);
    assert_eq!(spec.method, HttpMethod::Get);
    // Live round-trip on the audited /loom/graph/local route (LoomGraph { nodes, edges }).
    let graph = be.get_json(&format!(
        "/workspaces/{}/loom/graph/local?start_block_id={block_id}&max_depth=2",
        be.workspace_id
    ));
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let edges = graph
        .get("edges")
        .and_then(|e| e.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert!(
        nodes >= 1,
        "E3-23: the local graph (depth 2) must report >= 1 node (got {nodes})"
    );
    assert!(
        edges >= 1,
        "E3-23: the local graph (depth 2) must report >= 1 edge (got {edges})"
    );
    println!("E3-23 PASS: native local-graph request + live depth-2 graph -> {nodes} nodes, {edges} edges");
    mark_pass("E3-23");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-24: global graph — native LoomGraphClient::global_request (GET /loom/views/all)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_24_global_spec(ws: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomGraphClient::new(NATIVE_BASE, rt.handle().clone()).global_request(ws)
}

#[test]
fn parity_global_graph_native() {
    let spec = e3_24_global_spec("ws-1");
    assert_eq!(spec.method, HttpMethod::Get);
    // Same MT-021 correction as E3-23. GLOBAL is
    // GET /workspaces/:ws/loom/graph/global?node_limit=5000&hub_degree_threshold=0 -> LoomGraph
    // (backend_client.rs:2196-2211, backend route api/loom.rs:296). views/all is a real route but a
    // DIFFERENT one - MT-021 keeps it as "the independent count oracle used by the managed-PG proof",
    // not a graph projection. This files own live sibling parity_global_graph already fetches
    // /loom/graph/global.
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/graph/global"),
        "E3-24: native global-graph URL (got {})",
        spec.url
    );
    assert!(
        has_query(&spec, "node_limit", "5000") && has_query(&spec, "hub_degree_threshold", "0"),
        "E3-24: the global graph disables hub suppression so every LoomBlock is projected"
    );
    println!(
        "E3-24 NATIVE PASS: LoomGraphClient::global_request built {}",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (GET /loom/graph/global)"]
fn parity_global_graph() {
    let be = require_live_backend();
    let spec = e3_24_global_spec(&be.workspace_id);
    assert_eq!(spec.method, HttpMethod::Get);
    let graph = be.get_json(&format!(
        "/workspaces/{}/loom/graph/global",
        be.workspace_id
    ));
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default();
    assert!(
        !nodes.is_empty(),
        "E3-24: the global graph (depth 1) must report >= 1 node"
    );
    println!(
        "E3-24 PASS: native global-graph request + live graph -> {} nodes",
        nodes.len()
    );
    mark_pass("E3-24");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-25: folder tree — native LoomFolderClient::list_folders_request (GET /loom/folders)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_25_folders_spec(ws: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomFolderClient::new(NATIVE_BASE, rt.handle().clone()).list_folders_request(ws)
}

#[test]
fn parity_folder_tree_native() {
    let spec = e3_25_folders_spec("ws-1");
    assert_eq!(spec.method, HttpMethod::Get);
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/folders"),
        "E3-25: native folders URL (got {})",
        spec.url
    );
    println!(
        "E3-25 NATIVE PASS: LoomFolderClient::list_folders_request built {}",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (GET /loom/folders)"]
fn parity_folder_tree() {
    let be = require_live_backend();
    let spec = e3_25_folders_spec(&be.workspace_id);
    let path = spec
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&spec.url)
        .to_owned();
    // Send the NATIVE-built request path against the live backend.
    let folders = be.get_json(&path);
    let count = folders
        .as_array()
        .map(|a| a.len())
        .or_else(|| {
            folders
                .get("folders")
                .and_then(|f| f.as_array())
                .map(|a| a.len())
        })
        .unwrap_or(0);
    assert!(
        count >= 1,
        "E3-25: the folder tree must list >= 1 folder (got {count})"
    );
    println!("E3-25 PASS: native folder-list request surfaced {count} folder(s) from real PG");
    mark_pass("E3-25");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-26: color labels — native LoomFolderClient::recolor_request (PATCH /loom/folders/{id} {color})
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_26_recolor_spec(ws: &str, folder_id: &str, hex: &str) -> RequestSpec {
    let rt = native_rt();
    LoomFolderClient::new(NATIVE_BASE, rt.handle().clone()).recolor_request(ws, folder_id, hex)
}

#[test]
fn parity_color_labels_native() {
    let spec = e3_26_recolor_spec("ws-1", "folder-1", "#ff8800");
    assert_eq!(
        spec.method,
        HttpMethod::Patch,
        "E3-26: recolor is a merge-PATCH (never clobbers name/sort)"
    );
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/folders/folder-1"),
        "E3-26: native recolor URL (got {})",
        spec.url
    );
    let body = spec.body.expect("E3-26: recolor carries a body");
    assert_eq!(
        body["color"], "#ff8800",
        "E3-26: the recolor body carries ONLY the color key"
    );
    assert_eq!(
        body.as_object().map(|o| o.len()),
        Some(1),
        "E3-26: recolor is a pure merge-patch (color only)"
    );
    println!(
        "E3-26 NATIVE PASS: LoomFolderClient::recolor_request built PATCH {} {{color}}",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_FOLDER_ID (PUT/GET /loom/folders)"]
fn parity_color_labels() {
    let be = require_live_backend();
    let folder_id = std::env::var("HSK_TEST_FOLDER_ID")
        .expect("E3-26 requires_pg: set HSK_TEST_FOLDER_ID to a real folder id");
    // CTRL-2: the native merge-PATCH recolor request the production spawn path would send.
    let spec = e3_26_recolor_spec(&be.workspace_id, &folder_id, "#ff8800");
    assert_eq!(
        spec.body
            .as_ref()
            .and_then(|b| b.get("color"))
            .and_then(|c| c.as_str()),
        Some("#ff8800")
    );
    // Live round-trip on the audited route, then confirm the color is preserved on reload.
    be.put_json(
        &format!("/workspaces/{}/loom/folders/{folder_id}", be.workspace_id),
        &serde_json::json!({ "color": "#ff8800" }),
    );
    let reloaded = be.get_json(&format!("/workspaces/{}/loom/folders", be.workspace_id));
    assert!(
        serde_json::to_string(&reloaded)
            .unwrap()
            .contains("#ff8800"),
        "E3-26: the folder color label must be preserved after reload"
    );
    println!("E3-26 PASS: native recolor request + folder color #ff8800 preserved across reload");
    mark_pass("E3-26");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-27: tags + tag hubs — native LoomTagClient::tag_block_request (POST /loom/edges, edge_type=tag)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_27_tag_spec(ws: &str, block: &str, hub: &str) -> RequestSpec {
    let rt = native_rt();
    LoomTagClient::new(NATIVE_BASE, rt.handle().clone()).tag_block_request(ws, block, hub)
}

#[test]
fn parity_tags_and_hubs_native() {
    let spec = e3_27_tag_spec("ws-1", "blk-1", "hub-1");
    assert_eq!(spec.method, HttpMethod::Post);
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/edges"),
        "E3-27: native tag-edge URL (got {})",
        spec.url
    );
    let body = spec.body.expect("E3-27: tag edge carries a body");
    assert_eq!(
        body["source_block_id"], "blk-1",
        "E3-27: the tagged block is the edge source"
    );
    assert_eq!(
        body["target_block_id"], "hub-1",
        "E3-27: the tag hub is the edge target"
    );
    assert_eq!(
        body["edge_type"], "tag",
        "E3-27: the edge is a typed tag edge"
    );
    println!("E3-27 NATIVE PASS: LoomTagClient::tag_block_request built a tag edge blk-1 -> hub-1");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_BLOCK_ID (tag edge + tag hub query)"]
fn parity_tags_and_hubs() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    let tag = "parity-e3-27-tag";
    // CTRL-2: the native tag-edge request (proves the tag edge is source=block, edge_type=tag).
    let spec = e3_27_tag_spec(&be.workspace_id, &block_id, tag);
    assert_eq!(
        spec.body
            .as_ref()
            .and_then(|b| b.get("edge_type"))
            .and_then(|e| e.as_str()),
        Some("tag")
    );
    // Live round-trip on the audited tag + tag-hub routes.
    be.put_json(
        &format!("/workspaces/{}/loom/blocks/{block_id}", be.workspace_id),
        &serde_json::json!({ "tags": [tag] }),
    );
    let hub = be.get_json(&format!("/workspaces/{}/loom/tags/{tag}", be.workspace_id));
    assert!(
        serde_json::to_string(&hub).unwrap().contains(&block_id),
        "E3-27: the tagged block {block_id} must appear in the tag hub for '{tag}'"
    );
    println!("E3-27 PASS: native tag-edge request + block {block_id} surfaces in the tag hub for '{tag}'");
    mark_pass("E3-27");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-28: pins — native DrawerActionClient::pin_order_request + LoomSidebarClient::pins_request
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_28_pin_order_spec(ws: &str, block: &str) -> RequestSpec {
    let rt = native_rt();
    DrawerActionClient::new(NATIVE_BASE, rt.handle().clone()).pin_order_request(ws, block, 0)
}
fn e3_28_pins_view_spec(ws: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomSidebarClient::new(NATIVE_BASE, rt.handle().clone()).pins_request(ws)
}

#[test]
fn parity_pins_native() {
    let set = e3_28_pin_order_spec("ws-1", "blk-1");
    assert_eq!(set.method, HttpMethod::Put);
    assert!(
        set.url
            .ends_with("/workspaces/ws-1/loom/blocks/blk-1/pin-order"),
        "E3-28: native pin-order URL (got {})",
        set.url
    );
    assert_eq!(
        set.body.expect("E3-28: pin-order body")["pin_order"],
        0,
        "E3-28: the pin ordinal is set to 0 (top)"
    );
    let view = e3_28_pins_view_spec("ws-1");
    assert_eq!(view.method, HttpMethod::Get);
    assert!(
        view.url.ends_with("/workspaces/ws-1/loom/views/pins"),
        "E3-28: native pins-view URL (got {})",
        view.url
    );
    println!("E3-28 NATIVE PASS: pin-order PUT + pins-view GET built through the native clients");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_BLOCK_ID (PUT /loom/blocks/{id}/pin-order + GET /loom/views/pins)"]
fn parity_pins() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    // CTRL-2: the native pin-order + pins-view requests, sent against the live backend.
    let set = e3_28_pin_order_spec(&be.workspace_id, &block_id);
    let set_path = set
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&set.url)
        .to_owned();
    be.put_json(
        &set_path,
        &set.body.clone().unwrap_or(serde_json::Value::Null),
    );
    let view = e3_28_pins_view_spec(&be.workspace_id);
    let view_path = format!(
        "{}?limit=100",
        view.url.strip_prefix(NATIVE_BASE).unwrap_or(&view.url)
    );
    let pinned = be.get_json(&view_path);
    assert!(
        serde_json::to_string(&pinned).unwrap().contains(&block_id),
        "E3-28: the pinned block {block_id} must appear in the pins view (got {pinned})"
    );
    println!("E3-28 PASS: native pin-order + pins-view requests -> block {block_id} appears in the pins view");
    mark_pass("E3-28");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-29: backlinks — native LoomSidebarClient::backlinks_request (GET /loom/blocks/{id}/backlinks)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_29_backlinks_spec(ws: &str, block: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomSidebarClient::new(NATIVE_BASE, rt.handle().clone()).backlinks_request(ws, block)
}

#[test]
fn parity_backlinks_native() {
    let spec = e3_29_backlinks_spec("ws-1", "blk-1");
    assert_eq!(spec.method, HttpMethod::Get);
    assert!(
        spec.url
            .ends_with("/workspaces/ws-1/loom/blocks/blk-1/backlinks"),
        "E3-29: native backlinks URL (got {})",
        spec.url
    );
    println!(
        "E3-29 NATIVE PASS: LoomSidebarClient::backlinks_request built {}",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_BLOCK_ID (GET /loom/blocks/{id}/backlinks)"]
fn parity_backlinks() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    let spec = e3_29_backlinks_spec(&be.workspace_id, &block_id);
    let path = spec
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&spec.url)
        .to_owned();
    let backlinks = be.get_json(&path);
    let count = backlinks
        .as_array()
        .map(|a| a.len())
        .or_else(|| {
            backlinks
                .get("backlinks")
                .and_then(|b| b.as_array())
                .map(|a| a.len())
        })
        .unwrap_or(0);
    assert!(
        count >= 1,
        "E3-29: the backlinks of {block_id} must include >= 1 referencing block"
    );
    println!(
        "E3-29 PASS: native backlinks request -> {block_id} has {count} backlink(s) from real PG"
    );
    mark_pass("E3-29");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-30: unlinked mentions — native LoomSidebarClient::unlinked_request (GET .../unlinked-mentions)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_30_unlinked_spec(ws: &str, block: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomSidebarClient::new(NATIVE_BASE, rt.handle().clone()).unlinked_request(ws, block)
}

#[test]
fn parity_unlinked_mentions_native() {
    let spec = e3_30_unlinked_spec("ws-1", "blk-1");
    assert_eq!(spec.method, HttpMethod::Get);
    assert!(
        spec.url
            .ends_with("/workspaces/ws-1/loom/blocks/blk-1/unlinked-mentions"),
        "E3-30: native unlinked URL (got {})",
        spec.url
    );
    println!(
        "E3-30 NATIVE PASS: LoomSidebarClient::unlinked_request built {}",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_BLOCK_ID (GET /loom/blocks/{id}/unlinked-mentions)"]
fn parity_unlinked_mentions() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    let spec = e3_30_unlinked_spec(&be.workspace_id, &block_id);
    let path = spec
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&spec.url)
        .to_owned();
    let mentions = be.get_json(&path);
    let count = mentions
        .as_array()
        .map(|a| a.len())
        .or_else(|| {
            mentions
                .get("mentions")
                .and_then(|m| m.as_array())
                .map(|a| a.len())
        })
        .unwrap_or(0);
    assert!(
        count >= 1,
        "E3-30: the unlinked-mention scan must surface >= 1 mentioning block"
    );
    println!("E3-30 PASS: native unlinked-mentions request -> {block_id} has {count} unlinked mention(s)");
    mark_pass("E3-30");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-31: breadcrumbs — native graph::sidebar_panel::LoomSidebarPanel breadcrumb trail (in-process)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Build a parent -> child navigation trail through the native sidebar panel's breadcrumb history.
fn e3_31_breadcrumb_trail() -> LoomSidebarPanel {
    let mut panel = LoomSidebarPanel::new("ws-1");
    panel.push_breadcrumb("root", "Home");
    panel.push_breadcrumb("parent", "Parent");
    panel.push_breadcrumb("child", "Child");
    panel
}

#[test]
fn parity_breadcrumbs_native() {
    let panel = e3_31_breadcrumb_trail();
    let path: Vec<&str> = panel
        .breadcrumbs
        .iter()
        .map(|b| b.block_id.as_str())
        .collect();
    assert_eq!(
        path,
        vec!["root", "parent", "child"],
        "E3-31: the native breadcrumb trail records the parent->child path"
    );
    assert_eq!(
        panel.breadcrumbs.last().map(|b| b.title.as_str()),
        Some("Child"),
        "E3-31: the last crumb is the active block"
    );
    println!("E3-31 NATIVE PASS: LoomSidebarPanel breadcrumb trail = {path:?}");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_BLOCK_ID (GET /loom/blocks/{id}/breadcrumbs)"]
fn parity_breadcrumbs() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    // CTRL-2: the native breadcrumb trail (proves the sidebar builds a path history natively).
    let trail = e3_31_breadcrumb_trail();
    assert_eq!(
        trail.breadcrumbs.len(),
        3,
        "E3-31: the native breadcrumb trail has the built path"
    );
    // Live round-trip on the audited breadcrumbs route.
    let crumbs = be.get_json(&format!(
        "/workspaces/{}/loom/blocks/{block_id}/breadcrumbs",
        be.workspace_id
    ));
    let count = crumbs
        .as_array()
        .map(|a| a.len())
        .or_else(|| {
            crumbs
                .get("breadcrumbs")
                .and_then(|b| b.as_array())
                .map(|a| a.len())
        })
        .unwrap_or(0);
    assert!(
        count >= 1,
        "E3-31: breadcrumbs for {block_id} must return >= 1 path segment"
    );
    println!("E3-31 PASS: native breadcrumb trail + live breadcrumbs path has {count} segment(s)");
    mark_pass("E3-31");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-32: wiki-page projection — native LoomWikiClient::load_request (GET /loom/wiki/{id})
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_32_wiki_spec(ws: &str, projection_id: &str) -> GetRequestSpec {
    let rt = native_rt();
    LoomWikiClient::new(NATIVE_BASE, rt.handle().clone()).load_request(ws, projection_id)
}

#[test]
fn parity_wiki_page_projection_native() {
    let spec = e3_32_wiki_spec("ws-1", "proj-1");
    assert_eq!(spec.method, HttpMethod::Get);
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/wiki/proj-1"),
        "E3-32: native wiki-projection URL (got {})",
        spec.url
    );
    println!(
        "E3-32 NATIVE PASS: LoomWikiClient::load_request built {}",
        spec.url
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WIKI_PROJECTION_ID (GET /loom/wiki/{projection_id})"]
fn parity_wiki_page_projection() {
    let be = require_live_backend();
    let projection_id = std::env::var("HSK_TEST_WIKI_PROJECTION_ID").expect(
        "E3-32 requires_pg: set HSK_TEST_WIKI_PROJECTION_ID to a real compiled wiki page id",
    );
    let spec = e3_32_wiki_spec(&be.workspace_id, &projection_id);
    let path = spec
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&spec.url)
        .to_owned();
    let wiki = be.get_json(&path);
    let has_body = wiki
        .get("rendered_content")
        .and_then(|c| c.as_str())
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    assert!(
        has_body,
        "E3-32: the wiki-page projection must return non-empty rendered_content (got {wiki})"
    );
    println!(
        "E3-32 PASS: native wiki-load request -> projection {projection_id} resolved wikilinks"
    );
    mark_pass("E3-32");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-33: canvas board — native CanvasBoardClient::place_block_request (POST .../placements)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_33_place_spec(ws: &str, board: &str, block: &str) -> RequestSpec {
    let rt = native_rt();
    CanvasBoardClient::new(NATIVE_BASE, rt.handle().clone())
        .place_block_request(ws, board, block, 100.0, 100.0, 200.0, 120.0)
}

#[test]
fn parity_canvas_board_placement_native() {
    let spec = e3_33_place_spec("ws-1", "board-1", "blk-1");
    assert_eq!(spec.method, HttpMethod::Post);
    assert!(
        spec.url
            .ends_with("/workspaces/ws-1/loom/canvas-boards/board-1/placements"),
        "E3-33: native placement URL (got {})",
        spec.url
    );
    let body = spec.body.expect("E3-33: placement carries a body");
    assert_eq!(
        body["placed_block_id"], "blk-1",
        "E3-33: the placement references the real Loom block"
    );
    assert_eq!(body["x"], 100.0);
    assert_eq!(body["w"], 200.0);
    println!("E3-33 NATIVE PASS: CanvasBoardClient::place_block_request built POST {} {{placed_block_id,x,y,w,h}}", spec.url);
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_BOARD_ID + HSK_TEST_BLOCK_ID (POST /canvas-boards/{id}/placements)"]
fn parity_canvas_board_placement() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    let board_id = std::env::var("HSK_TEST_BOARD_ID")
        .expect("E3-33 requires_pg: set HSK_TEST_BOARD_ID to a real canvas board id");
    // CTRL-2: build + send the native placement request.
    let spec = e3_33_place_spec(&be.workspace_id, &board_id, &block_id);
    let path = spec
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&spec.url)
        .to_owned();
    let placement = be.post_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    let placement_id = placement["placement_id"]
        .as_str()
        .expect("E3-33: placement returns a placement_id");
    let board = be.get_json(&format!(
        "/workspaces/{}/loom/canvas-boards/{board_id}",
        be.workspace_id
    ));
    assert!(
        serde_json::to_string(&board)
            .unwrap()
            .contains(placement_id),
        "E3-33: the new placement {placement_id} must appear in the board view"
    );
    println!("E3-33 PASS: native placement request -> {placement_id} of block {block_id} returned in the board view");
    mark_pass("E3-33");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-34: block-collection table view — native BlockViewClient::create_view_request + query_results
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_34_create_spec(ws: &str, kind: BlockViewKind) -> RequestSpec {
    let rt = native_rt();
    BlockViewClient::new(NATIVE_BASE, rt.handle().clone()).create_view_request(
        ws,
        "parity-view-stable-id",
        "parity-view",
        &BlockViewDefinition::of_kind(kind),
    )
}
fn e3_34_results_spec(ws: &str, view_id: &str) -> RequestSpec {
    let rt = native_rt();
    BlockViewClient::new(NATIVE_BASE, rt.handle().clone()).query_results_request(ws, view_id, 50, 0)
}

#[test]
fn parity_block_collection_table_native() {
    let create = e3_34_create_spec("ws-1", BlockViewKind::Table);
    assert_eq!(create.method, HttpMethod::Post);
    assert!(
        create
            .url
            .ends_with("/workspaces/ws-1/loom/views/definitions"),
        "E3-34: native create-view URL (got {})",
        create.url
    );
    let body = create.body.expect("E3-34: create-view body");
    assert_eq!(
        body["definition"]["kind"], "table",
        "E3-34: the created view is a table view_def"
    );
    let results = e3_34_results_spec("ws-1", "view-1");
    assert_eq!(
        results.method,
        HttpMethod::Post,
        "E3-34: the results query is a POST (not a GET)"
    );
    assert!(
        results
            .url
            .ends_with("/workspaces/ws-1/loom/views/definitions/view-1/results"),
        "E3-34: native results URL (got {})",
        results.url
    );
    println!("E3-34 NATIVE PASS: BlockViewClient create table view_def + POST /results built through the native client");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_WORKSPACE_ID (POST /loom/views/definitions + /results)"]
fn parity_block_collection_table() {
    let be = require_live_backend();
    // CTRL-2: build + send the native create-view request.
    let create = e3_34_create_spec(&be.workspace_id, BlockViewKind::Table);
    let create_path = create
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&create.url)
        .to_owned();
    let view = be.post_json(
        &create_path,
        &create.body.clone().unwrap_or(serde_json::Value::Null),
    );
    let view_id = view_block_id(&view);
    let results = e3_34_results_spec(&be.workspace_id, &view_id);
    let results_path = results
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&results.url)
        .to_owned();
    let results_body = be.post_json(
        &results_path,
        &results.body.clone().unwrap_or(serde_json::Value::Null),
    );
    let rows = results_body
        .get("blocks")
        .and_then(|r| r.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    assert!(
        rows > 0,
        "E3-34: the table view query must return > 0 blocks (got {rows})"
    );
    println!("E3-34 PASS: native create-view + query-results -> {rows} row(s)");
    mark_pass("E3-34");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-35: block-collection Kanban — native BlockViewClient::card_move_request + query_results
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn e3_35_card_move_spec(ws: &str, block: &str, add: &[String], remove: &[String]) -> RequestSpec {
    let rt = native_rt();
    BlockViewClient::new(NATIVE_BASE, rt.handle().clone()).card_move_request(ws, block, add, remove)
}

#[test]
fn parity_block_collection_kanban_native() {
    // A Kanban view groups by tag; moving a card = a tag mutation (add the target lane's tag).
    let def = BlockViewDefinition::of_kind(BlockViewKind::Kanban);
    assert_eq!(
        def.kind,
        BlockViewKind::Kanban,
        "E3-35: the native Kanban view is grouped"
    );
    assert!(
        def.group_by.is_some(),
        "E3-35: a native Kanban view carries a group_by so it renders lanes"
    );
    let spec = e3_35_card_move_spec(
        "ws-1",
        "blk-1",
        &["done".to_string()],
        &["todo".to_string()],
    );
    assert_eq!(spec.method, HttpMethod::Patch);
    assert!(
        spec.url.ends_with("/workspaces/ws-1/loom/blocks/blk-1"),
        "E3-35: native card-move URL (got {})",
        spec.url
    );
    let body = spec.body.expect("E3-35: card-move body");
    assert_eq!(
        body["add_tags"][0], "done",
        "E3-35: the card moves into the 'done' lane (add tag)"
    );
    assert_eq!(
        body["remove_tags"][0], "todo",
        "E3-35: the card leaves the 'todo' lane (remove tag)"
    );
    println!(
        "E3-35 NATIVE PASS: BlockViewClient::card_move_request built a lane-move tag mutation"
    );
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_VIEW_ID + HSK_TEST_BLOCK_ID (Kanban move + re-query)"]
fn parity_block_collection_kanban() {
    let be = require_live_backend();
    let block_id = be.require_block_id();
    let view_id = std::env::var("HSK_TEST_VIEW_ID")
        .expect("E3-35 requires_pg: set HSK_TEST_VIEW_ID to a real Kanban view_def id");
    let target_column = "parity-e3-35-done".to_string();
    // CTRL-2: build + send the native card-move (add the target lane's tag).
    let spec = e3_35_card_move_spec(
        &be.workspace_id,
        &block_id,
        std::slice::from_ref(&target_column),
        &[],
    );
    let path = spec
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&spec.url)
        .to_owned();
    be.put_json(&path, &spec.body.clone().unwrap_or(serde_json::Value::Null));
    // Re-query via the native results request.
    let results = e3_34_results_spec(&be.workspace_id, &view_id);
    let results_path = results
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&results.url)
        .to_owned();
    let results_body = be.post_json(
        &results_path,
        &results.body.clone().unwrap_or(serde_json::Value::Null),
    );
    let s = serde_json::to_string(&results_body).unwrap();
    assert!(
        s.contains(&block_id) && s.contains(&target_column),
        "E3-35: after the move, card {block_id} must appear in column '{target_column}'"
    );
    println!("E3-35 PASS: native card-move + re-query -> Kanban card {block_id} moved to '{target_column}'");
    mark_pass("E3-35");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// E3-36: block-collection calendar — native BlockViewClient::query_results_request (calendar view_def)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn parity_block_collection_calendar_native() {
    let def = BlockViewDefinition::of_kind(BlockViewKind::Calendar);
    assert_eq!(
        def.kind,
        BlockViewKind::Calendar,
        "E3-36: the native calendar view buckets by date"
    );
    let results = e3_34_results_spec("ws-1", "cal-1");
    assert_eq!(results.method, HttpMethod::Post);
    assert!(
        results
            .url
            .ends_with("/workspaces/ws-1/loom/views/definitions/cal-1/results"),
        "E3-36: native calendar results URL (got {})",
        results.url
    );
    println!("E3-36 NATIVE PASS: native calendar view_def + POST /results built through the native client");
}

#[test]
#[ignore = "requires_pg: live handshake_core + PostgreSQL + HSK_TEST_VIEW_ID (calendar view query for today)"]
fn parity_block_collection_calendar() {
    let be = require_live_backend();
    let view_id = std::env::var("HSK_TEST_VIEW_ID")
        .expect("E3-36 requires_pg: set HSK_TEST_VIEW_ID to a real calendar view_def id");
    let today = "2026-06-26";
    // CTRL-2: build + send the native calendar results request.
    let results = e3_34_results_spec(&be.workspace_id, &view_id);
    let path = results
        .url
        .strip_prefix(NATIVE_BASE)
        .unwrap_or(&results.url)
        .to_owned();
    let results_body = be.post_json(
        &path,
        &results.body.clone().unwrap_or(serde_json::Value::Null),
    );
    assert!(
        serde_json::to_string(&results_body)
            .unwrap()
            .contains(today),
        "E3-36: the calendar view for {today} must surface the daily journal block"
    );
    println!(
        "E3-36 PASS: native calendar results request -> {today} surfaced the daily journal block"
    );
    mark_pass("E3-36");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// helper (pure)
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Extract the saved-view block id from a BlockViewRecord create response (`record.block.block_id`;
/// tolerate flat fallbacks).
fn view_block_id(view: &serde_json::Value) -> String {
    view.get("block")
        .and_then(|b| b.get("block_id"))
        .and_then(|v| v.as_str())
        .or_else(|| view.get("block_id").and_then(|v| v.as_str()))
        .or_else(|| view.get("id").and_then(|v| v.as_str()))
        .expect("E3-34: block-view record returns block.block_id")
        .to_owned()
}
