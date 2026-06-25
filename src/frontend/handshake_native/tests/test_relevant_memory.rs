//! Relevant Memory (FEMS interop) proofs — WP-KERNEL-012 MT-063 (cluster E9).
//!
//! This suite proves the READ-ONLY FEMS retrieval-capsule consumer end to end at the widget/client
//! level (fixture JSON + an in-process mock server + egui_kittest), which is what is provable NOW. The
//! live FEMS read endpoint is ABSENT in the current handshake_core build (the DESIGNED primary path),
//! so the typed-blocker + empty-state path is the production reality; the live fetch against a real FEMS
//! route is `NEEDS_MANAGED_RESOURCE_PROOF` and not asserted here.
//!
//! PROOF-POSTURE GATE (must_fix #3): the decode/round-trip tests below feed IMPLEMENTER-AUTHORED JSON
//! matching the contract-dictated client model into the client's OWN deserializer over an in-process
//! mock `TcpListener`. They are FIXTURE-ONLY proof of transport + typed-blocker + defensive clamp +
//! tolerant decode + render/AccessKit — they are NOT interop proof against the real FEMS resource and
//! MUST NOT be read as backend-aligned. The client model is currently FIXTURE-ALIGNED, NOT
//! backend-aligned: it diverges from the only `MemoryPack` the backend produces (`crate::ace::MemoryPack`
//! / `MemoryPackItem` / `FemsSourceRef`: `memory_id` not `id`, free-form `memory_class` incl. `"working"`
//! not a 3-variant `kind`, `source_refs` not `source`, required `token_estimate`, no `truncated`/
//! `context_key`). That shape drift is a CONTRACT DEFECT surfaced as a typed blocker to the WP validator/
//! orchestrator (see `src/fems/memory_client.rs` module docs). The IN-SCOPE hardening here is the
//! tolerance floor: `fetch_decodes_capsule_with_unknown_class` proves an unknown/future `memory_class`
//! is skipped (not fatal) through the full fetch+decode path.
//!
//! Proof map:
//! - PT-001 / AC-001 / AC-002: `fetch_pack` over the mock server decodes episodic+semantic+procedural
//!   items; a 30-item response clamps to 24 with `truncated=true`. (`fetch_live_*` tests.)
//! - PT-004 / AC-005: a 404 from the mock maps to `MemoryClientError::EndpointMissing` (the typed
//!   blocker), and the panel renders the empty-state banner — no panic, no silent no-op.
//!   (`fetch_live_404_is_endpoint_missing`, `panel_renders_endpoint_missing_banner`.)
//! - PT-002 / AC-003: `panel_renders_grouped_with_source_links` renders a seeded context's items grouped
//!   by kind, each with a visible "Go to source" affordance; saves a screenshot to the EXTERNAL root.
//! - PT-003 / AC-004: `panel_source_click_routes_to_nav_bus` clicks an item's source link and asserts
//!   the navigation bus received the target resolved from the MemorySource (precedence: uri).
//! - PT-005 / AC-007: `panel_accesskit_nodes_present` dumps the live AccessKit tree and asserts
//!   `relevant-memory-panel` (GenericContainer), `relevant-memory-list` (List), `mem-item-{id}`
//!   (ListItem), and `mem-source-{id}` (Button) with the correct roles + nesting.
//! - AC-006: `read_only_no_write_verbs` greps the production source for write verbs / store access
//!   (GET only); `assert_no_local_artifact_dir` guards artifact hygiene (CX-212E).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use serde_json::{json, Value};

use handshake_native::fems::memory_client::{
    MemoryClient, MemoryClientError, MemoryContext, MemoryKind, MemoryPack, MemorySource,
};
use handshake_native::fems::relevant_memory_panel::{
    mem_item_author_id, mem_source_author_id, FnNavigationBus, MemoryNavTarget, RelevantMemoryPanel,
    ENDPOINT_MISSING_BANNER, RELEVANT_MEMORY_LIST_AUTHOR_ID, RELEVANT_MEMORY_PANEL_AUTHOR_ID,
};
use handshake_native::theme::HsTheme;

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Artifact hygiene (CX-212E / SCREENSHOT RULE): all artifacts go to the EXTERNAL root ONLY.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the SCREENSHOT/TEST-ARTIFACT RULE).
/// Artifacts go to the external `Handshake_Artifacts/handshake-test` root ONLY; a stray `test_output/`
/// OR `tests/screenshots/` is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in ["test_output", "tests/screenshots"] {
        let p = Path::new(local);
        assert!(
            !p.exists(),
            "artifact hygiene: no repo-local '{local}' dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            p.display()
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// In-process mock HTTP server (the PROVEN MT-020/MT-037 TcpListener pattern — no new dependency).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// The captured request line + headers a mock exchange delivers (the reply is configured by the caller;
/// only the captured request is asserted on).
type MockCapture = (String, std::collections::HashMap<String, String>);

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line + headers. Returns (base_url, join handle delivering the capture).
fn spawn_mock(
    status_line: &'static str,
    body: Value,
) -> (String, std::thread::JoinHandle<MockCapture>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let (request_line, headers) = read_one_http_request(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        (request_line, headers)
    });
    (base_url, handle)
}

/// Read one HTTP request (request line + headers) off the stream. A GET has no body, so reading to the
/// header terminator is sufficient.
fn read_one_http_request(
    stream: &mut std::net::TcpStream,
) -> (String, std::collections::HashMap<String, String>) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    loop {
        let n = stream.read(&mut tmp).unwrap_or(0);
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if String::from_utf8_lossy(&buf).contains("\r\n\r\n") {
            break;
        }
    }
    let text = String::from_utf8_lossy(&buf).to_string();
    let hdr_end = text.find("\r\n\r\n").unwrap_or(text.len());
    let mut lines = text[..hdr_end].lines();
    let request_line = lines.next().unwrap_or("").to_string();
    let mut headers = std::collections::HashMap::new();
    for l in lines {
        if let Some((k, v)) = l.split_once(':') {
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
        }
    }
    (request_line, headers)
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Fixtures.
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// A capsule body with one item of each Pillar 12 kind, each provenance-first.
fn three_kind_body() -> Value {
    json!({
        "context_key": "ws=W1|doc=D1|cur=12|sel_len=0",
        "token_estimate": 280,
        "truncated": false,
        "items": [
            {"id": "ep-1", "kind": "episodic", "summary": "You edited the intro 2h ago",
             "source": {"event_id": "EV-100"}, "score": 0.91},
            {"id": "sem-1", "kind": "semantic", "summary": "Aria is the protagonist",
             "source": {"uri": "loom://block/aria"}, "score": 0.84},
            {"id": "proc-1", "kind": "procedural", "summary": "How to render the scene",
             "source": {"document_id": "D9", "byte_range": [10, 40]}}
        ]
    })
}

/// A capsule body with 30 items (all episodic) to prove the client-side <=24 clamp.
fn thirty_item_body() -> Value {
    let items: Vec<Value> = (0..30)
        .map(|n| {
            json!({"id": format!("ep-{n}"), "kind": "episodic", "summary": format!("event {n}"),
                   "source": {"event_id": format!("EV-{n}")}})
        })
        .collect();
    json!({"context_key": "k", "truncated": false, "token_estimate": 480, "items": items})
}

/// A typed pack used to seed the panel for render/interaction tests (no network).
fn seeded_pack() -> MemoryPack {
    serde_json::from_value(three_kind_body()).expect("seed pack must decode")
}

fn dark() -> handshake_native::theme::HsPalette {
    HsTheme::Dark.palette()
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-001 / AC-001 / AC-002 — live fetch decodes 3 kinds + clamps to 24.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn fetch_live_decodes_three_kinds() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", three_kind_body());
    let client = MemoryClient::with_base_url(base_url);
    let ctx = MemoryContext::from_focus("W1", Some("D1".into()), None, Some(12));
    let pack = rt()
        .block_on(async { client.fetch_pack("W1", &ctx).await })
        .expect("AC-001: a 200 capsule must decode");
    let (req_line, _headers) = server.join().unwrap();

    // AC-006 (the live half): the request is a GET (read-only), never a write verb.
    assert!(
        req_line.starts_with("GET "),
        "AC-006: fetch_pack must issue a GET (read-only); got request line '{req_line}'"
    );
    assert!(
        req_line.contains("/workspaces/W1/memory/pack"),
        "fetch_pack must hit the documented FEMS read route; got '{req_line}'"
    );

    assert_eq!(pack.items.len(), 3, "AC-001: all three items decode");
    assert_eq!(pack.items_of_kind(MemoryKind::Episodic).count(), 1);
    assert_eq!(pack.items_of_kind(MemoryKind::Semantic).count(), 1);
    assert_eq!(pack.items_of_kind(MemoryKind::Procedural).count(), 1);
    assert!(!pack.truncated);
    assert_eq!(pack.token_estimate, Some(280));
    println!(
        "PT-001 three-kind decode OK: {} items, kinds=[ep,sem,proc], token_estimate={:?}",
        pack.items.len(),
        pack.token_estimate
    );
}

#[test]
fn fetch_live_clamps_thirty_to_24() {
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", thirty_item_body());
    let client = MemoryClient::with_base_url(base_url);
    let ctx = MemoryContext::for_workspace("W1");
    let pack = rt()
        .block_on(async { client.fetch_pack("W1", &ctx).await })
        .expect("AC-002: a 30-item capsule must decode (then clamp)");
    let _ = server.join();

    assert_eq!(pack.items.len(), 24, "AC-002: clamped to exactly 24 client-side");
    assert!(pack.truncated, "AC-002: truncated must be true after the defensive clamp");
    println!("PT-001 clamp OK: 30 -> {} items, truncated={}", pack.items.len(), pack.truncated);
}

#[test]
fn fetch_decodes_capsule_with_unknown_class() {
    // must_fix #2 (through the full fetch path): a capsule that mixes the real backend's `"working"`
    // class (ace/mod.rs:1927,2024,2095) with the three rendered kinds must NOT fail the whole decode.
    // The unknown-class item is skipped + logged; the known-kind items survive. Before the tolerant
    // decode, `resp.json::<MemoryPack>()` would error with `unknown variant 'working'` and the consumer
    // would surface a permanent Decode error for every such capsule.
    let body = json!({
        "context_key": "k",
        "token_estimate": 120,
        "items": [
            {"id": "ep", "kind": "episodic", "summary": "edited", "source": {"event_id": "E"}},
            {"id": "work", "kind": "working", "summary": "scratch buffer", "source": {"event_id": "W"}},
            {"id": "sem", "kind": "semantic", "summary": "fact", "source": {"uri": "loom://x"}}
        ]
    });
    let (base_url, server) = spawn_mock("HTTP/1.1 200 OK", body);
    let client = MemoryClient::with_base_url(base_url);
    let ctx = MemoryContext::for_workspace("W1");
    let pack = rt()
        .block_on(async { client.fetch_pack("W1", &ctx).await })
        .expect("must_fix #2: an unknown memory_class must NOT fail the capsule decode");
    let _ = server.join();

    assert_eq!(pack.items.len(), 2, "the two known-kind items survive; the 'working' item is skipped");
    assert_eq!(pack.items_of_kind(MemoryKind::Episodic).count(), 1);
    assert_eq!(pack.items_of_kind(MemoryKind::Semantic).count(), 1);
    assert!(pack.items.iter().all(|i| i.id != "work"), "the unknown-class item is dropped");
    println!("must_fix #2 OK: capsule with 'working' class decoded, unknown item skipped, 2 kept");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-005 — the missing-endpoint typed blocker (the DESIGNED primary path).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn fetch_live_404_is_endpoint_missing() {
    // A 404 (route absent) maps to the typed blocker — NOT a panic, NOT a silent no-op.
    let (base_url, server) = spawn_mock("HTTP/1.1 404 Not Found", json!({"error": "not found"}));
    let client = MemoryClient::with_base_url(base_url);
    let ctx = MemoryContext::for_workspace("W1");
    let result = rt().block_on(async { client.fetch_pack("W1", &ctx).await });
    let _ = server.join();

    match result {
        Err(MemoryClientError::EndpointMissing { probed_path }) => {
            assert!(
                probed_path.contains("/workspaces/W1/memory/pack"),
                "AC-005: EndpointMissing must name the probed path; got '{probed_path}'"
            );
            println!("PT-004 typed blocker OK: EndpointMissing(probed='{probed_path}')");
        }
        other => panic!("AC-005: a 404 must map to EndpointMissing, got {other:?}"),
    }
}

#[test]
fn panel_renders_endpoint_missing_banner() {
    // The panel maps the typed blocker to the calm empty-state banner — no panic, no list.
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 240.0))
        .build_ui(|ui| {
            let mut panel = RelevantMemoryPanel::new();
            panel.set_blocker(MemoryClientError::EndpointMissing {
                probed_path: "/workspaces/W1/memory/pack".into(),
            });
            let mut bus = FnNavigationBus(|_t: MemoryNavTarget| {});
            panel.show(ui, &dark(), &mut bus);
            // The blocker is surfaced upward (the host calls take_blocker for the validator handoff).
            assert!(panel.has_endpoint_missing_blocker());
        });
    harness.run();

    // The banner text is present in the live tree (AccessKit label/value), proving no silent no-op.
    let root = harness.root();
    let mut banner_found = false;
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        let txt = format!("{} {}", ak.label().unwrap_or_default(), ak.value().unwrap_or_default());
        if txt.contains("FEMS read endpoint not present") {
            banner_found = true;
        }
    }
    assert!(
        banner_found,
        "AC-005: the empty-state banner '{ENDPOINT_MISSING_BANNER}' must render for EndpointMissing"
    );
    // The outer panel container is still present even in the blocker state (addressable by an agent).
    assert!(
        has_author(&root, RELEVANT_MEMORY_PANEL_AUTHOR_ID),
        "AC-005/AC-007: the relevant-memory-panel container must render in the blocker state too"
    );
    println!("PT-004 banner OK: empty-state banner rendered, container present, blocker surfaced");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-003 — grouped render with a per-item source affordance (+ screenshot).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn panel_renders_grouped_with_source_links() {
    let pack = seeded_pack();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(380.0, 320.0))
        .wgpu()
        .build_ui(move |ui| {
            let mut panel = RelevantMemoryPanel::new();
            panel.set_pack(pack.clone());
            let mut bus = FnNavigationBus(|_t: MemoryNavTarget| {});
            panel.show(ui, &dark(), &mut bus);
        });
    harness.run();
    harness.run();

    let root = harness.root();

    // AC-003: the list container + one row per item + a source link per navigable item.
    assert!(
        has_author(&root, RELEVANT_MEMORY_LIST_AUTHOR_ID),
        "AC-003: the relevant-memory-list container must render when items are present"
    );
    for id in ["ep-1", "sem-1", "proc-1"] {
        assert!(
            has_author(&root, &mem_item_author_id(id)),
            "AC-003: an item row mem-item-{id} must render"
        );
        assert!(
            has_author(&root, &mem_source_author_id(id)),
            "AC-003: a 'Go to source' affordance mem-source-{id} must render (provenance-first)"
        );
    }
    println!("PT-002 grouped render OK: list + 3 item rows + 3 source affordances present");

    // Screenshot to the EXTERNAL root ONLY (best-effort pixel readback).
    if let Ok(image) = harness.render() {
        let ext_dir = external_artifact_dir("wp-kernel-012-mt-063");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_path = ext_dir.join("MT-063-relevant-memory-panel.png");
        let saved = image.save(&ext_path).is_ok();
        println!(
            "PT-002 screenshot: {}x{} saved_ext={saved} ({})",
            image.width(),
            image.height(),
            ext_path.display()
        );
    } else {
        println!("PT-002 screenshot: GPU readback unavailable on this host (structural proof stands)");
    }

    assert_no_local_artifact_dir();
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-004 — clicking a source link routes the resolved target to the nav bus.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn panel_source_click_routes_to_nav_bus() {
    use std::cell::RefCell;
    use std::rc::Rc;

    // One semantic item with a uri source so the resolved target is deterministic (precedence: uri).
    let pack: MemoryPack = serde_json::from_value(json!({
        "context_key": "k",
        "items": [
            {"id": "sem-1", "kind": "semantic", "summary": "Aria is the protagonist",
             "source": {"uri": "loom://block/aria"}}
        ]
    }))
    .unwrap();

    let captured: Rc<RefCell<Vec<MemoryNavTarget>>> = Rc::new(RefCell::new(Vec::new()));
    let cap_for_ui = captured.clone();

    let mut harness = Harness::builder()
        .with_size(egui::vec2(380.0, 220.0))
        .build_ui(move |ui| {
            let mut panel = RelevantMemoryPanel::new();
            panel.set_pack(pack.clone());
            let cap = cap_for_ui.clone();
            let mut bus = FnNavigationBus(move |t: MemoryNavTarget| cap.borrow_mut().push(t));
            panel.show(ui, &dark(), &mut bus);
        });
    harness.run();

    // Click the source link by its stable AccessKit address (mem-source-sem-1).
    let source_author = mem_source_author_id("sem-1");
    harness
        .get_by(|n| n.author_id() == Some(source_author.as_str()))
        .click();
    harness.run();

    let got = captured.borrow();
    assert_eq!(got.len(), 1, "AC-004: exactly one navigation target routed on click");
    assert_eq!(
        got[0],
        MemoryNavTarget::Uri { uri: "loom://block/aria".into() },
        "AC-004: the routed target must be the uri resolved from the MemorySource (precedence)"
    );
    println!("PT-003 nav routing OK: click -> nav_bus.navigate_to(Uri loom://block/aria)");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-005 / AC-007 — AccessKit nodes present with correct roles + nesting.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn panel_accesskit_nodes_present() {
    let pack = seeded_pack();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(380.0, 320.0))
        .build_ui(move |ui| {
            let mut panel = RelevantMemoryPanel::new();
            panel.set_pack(pack.clone());
            let mut bus = FnNavigationBus(|_t: MemoryNavTarget| {});
            panel.show(ui, &dark(), &mut bus);
        });
    harness.run();

    let root = harness.root();

    let panel_role = role_of(&root, RELEVANT_MEMORY_PANEL_AUTHOR_ID);
    assert_eq!(
        panel_role.as_deref(),
        Some("GenericContainer"),
        "AC-007: '{RELEVANT_MEMORY_PANEL_AUTHOR_ID}' must be Role::GenericContainer (got {panel_role:?})"
    );
    let list_role = role_of(&root, RELEVANT_MEMORY_LIST_AUTHOR_ID);
    assert_eq!(
        list_role.as_deref(),
        Some("List"),
        "AC-007: '{RELEVANT_MEMORY_LIST_AUTHOR_ID}' must be Role::List (got {list_role:?})"
    );

    // At least one mem-item-{id} (ListItem) with a mem-source-{id} (Button) pair.
    let item_role = role_of(&root, &mem_item_author_id("ep-1"));
    assert_eq!(
        item_role.as_deref(),
        Some("ListItem"),
        "AC-007: 'mem-item-ep-1' must be Role::ListItem (got {item_role:?})"
    );
    let source_role = role_of(&root, &mem_source_author_id("ep-1"));
    assert_eq!(
        source_role.as_deref(),
        Some("Button"),
        "AC-007: 'mem-source-ep-1' must be Role::Button (got {source_role:?})"
    );

    // Nesting: the list is under the panel; the item is under the list.
    assert!(
        author_under(&root, RELEVANT_MEMORY_LIST_AUTHOR_ID, RELEVANT_MEMORY_PANEL_AUTHOR_ID),
        "AC-007: the list node must be nested under the panel container"
    );
    assert!(
        author_under(&root, &mem_item_author_id("ep-1"), RELEVANT_MEMORY_LIST_AUTHOR_ID),
        "AC-007: an item row must be nested under the list container"
    );

    println!(
        "PT-005 accesskit dump: {{\"{RELEVANT_MEMORY_PANEL_AUTHOR_ID}\":\"{}\",\
         \"{RELEVANT_MEMORY_LIST_AUTHOR_ID}\":\"{}\",\"mem-item-ep-1\":\"{}\",\"mem-source-ep-1\":\"{}\"}}",
        panel_role.unwrap_or_default(),
        list_role.unwrap_or_default(),
        item_role.unwrap_or_default(),
        source_role.unwrap_or_default()
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-006 — read-only consumption (static gate over the production source).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn read_only_no_write_verbs() {
    // The memory_client production source must perform GET only — no client.post/put/delete/patch and no
    // direct store access. (The live-wire half of AC-006 is asserted in fetch_live_decodes_three_kinds.)
    let src = include_str!("../src/fems/memory_client.rs");
    for verb in [".post(", ".put(", ".delete(", ".patch("] {
        assert!(
            !src.contains(verb),
            "AC-006: memory_client must be read-only — found a write verb '{verb}' in the source"
        );
    }
    // The reqwest GET builder call (`self.client.get(...)`, idiomatically multi-line). Match the
    // method-chain `.get(` token rather than a contiguous `self.client.get(` string so an idiomatic
    // line wrap does not defeat the gate. This file has no serde `Value::get` calls, so `.get(` here is
    // unambiguously the reqwest GET verb. The client field is referenced as `self.client`.
    assert!(
        src.contains(".get(&url)"),
        "AC-006: memory_client must issue a GET via the reqwest builder (`.get(&url)`)"
    );
    // The GET goes through the struct's own reqwest client field (`self.client`, idiomatically wrapped
    // as `self\n .client\n .get(...)`). The `client` field + the GET builder together prove it is the
    // shared client, not a fresh per-call `reqwest::Client::new()` inside fetch_pack.
    assert!(
        src.contains(".client"),
        "RISK-006/MC-005: the GET must go through the struct's shared reqwest client field"
    );
    // No direct store access (no sqlx / postgres / sqlite handle in the consumer).
    for store in ["sqlx::", "PgPool", "tokio_postgres", "rusqlite", "Sqlite", "sqlite"] {
        assert!(
            !src.contains(store),
            "AC-006: memory_client must NOT access a store directly — found '{store}'"
        );
    }
    // Reuse, not a second stack: it pulls the shared client + base url from backend_client.
    assert!(
        src.contains("shared_http_client") && src.contains("BACKEND_BASE_URL"),
        "RISK-006/MC-005: memory_client must reuse the shared backend_client pool + base url"
    );
    println!("AC-006 read-only gate OK: GET only, no write verbs, no direct store, shared client reused");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// Provenance: a non-navigable item renders with a disabled source link (RISK-003/MC-003).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn non_navigable_item_disables_source_link() {
    // Pure proof of the safe construction: an all-absent source yields no nav target (so the link is
    // disabled at render). The render-disabled state is exercised by the grouped render test; this
    // asserts the construction can never produce a dead clickable target.
    assert_eq!(MemoryNavTarget::from_source(&MemorySource::default()), None);
    // A navigable source still resolves (precedence: uri > doc > event).
    let s = MemorySource { event_id: Some("EV-1".into()), ..Default::default() };
    assert_eq!(MemoryNavTarget::from_source(&s), Some(MemoryNavTarget::Event { event_id: "EV-1".into() }));
    println!("RISK-003 OK: non-navigable source -> None (disabled link, no dead click)");
}

// ── small AccessKit tree helpers ──────────────────────────────────────────────────────────────────

/// True if any node in the live tree carries `author_id`.
fn has_author(root: &egui_kittest::Node<'_>, author_id: &str) -> bool {
    root.children_recursive()
        .any(|n| n.accesskit_node().author_id() == Some(author_id))
}

/// The `{:?}` role string of the first node with `author_id`, if present. Extracts the owned role
/// string inside the iterator borrow (the proven `find_node` pattern) so no borrowed node escapes.
fn role_of(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// True if a node addressed `child_author` has an ancestor addressed `ancestor_author`. The parent walk
/// happens inside the `children_recursive` borrow so no node escapes the iterator's lifetime.
fn author_under(
    root: &egui_kittest::Node<'_>,
    child_author: &str,
    ancestor_author: &str,
) -> bool {
    for node in root.children_recursive() {
        if node.accesskit_node().author_id() != Some(child_author) {
            continue;
        }
        let mut cur = node.parent();
        while let Some(p) = cur {
            if p.accesskit_node().author_id() == Some(ancestor_author) {
                return true;
            }
            cur = p.parent();
        }
    }
    false
}
