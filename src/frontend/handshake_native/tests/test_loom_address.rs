//! WP-KERNEL-012 MT-032 PROOFS — "everything is a Loom block" addressing + live backlinks.
//!
//! Coverage map (each proof_target PT-* / acceptance_criteria AC-*):
//!   - PT-1 / AC-1: `loom_uri`/`parse_loom_uri` round-trip + ContentHash determinism — the standalone
//!     lib unit tests (`loom_address::tests`) carry these; re-asserted at the crate boundary here so a
//!     consumer-visible regression is caught (`loom_uri_round_trip_at_boundary`).
//!   - AC-4 / PT-4: clicking a backlink row fires the shared-bus OpenDocument command with the correct
//!     document id — kittest renders the EXISTING MT-015 backlinks panel, clicks an entry, routes the
//!     `EditorEvent::BacklinkActivated` through `dispatch_backlink_open`, and asserts the bus staged the
//!     right document id + the OpenDocument command is the dispatched one (`backlink_click_fires_open_document`).
//!   - AC-5 / AC-9: a canvas placement with a `placed_block_id` exposes its `loom://` chip as the
//!     placement node's AccessKit description (`canvas_node_loom_chip_in_accesskit`); a placement with an
//!     empty block id has NO chip (RISK-3, `empty_placement_has_no_loom_chip`).
//!   - AC-7 / PT-5: the EXISTING MT-015 backlinks panel exposes `backlinks-panel` (Group/List) + at least
//!     one `backlink-{id}` node when it has data (`backlinks_panel_accesskit_tree`).
//!   - PT-5 (graph): a graph node tooltip exposes its `loom://` URI + backlink count via the AccessKit
//!     description (`graph_node_loom_tooltip_in_accesskit`).
//!   - HBR-VIS: a `.wgpu()` screenshot of the canvas with a loom:// chip card, written EXTERNALLY
//!     (`canvas_loom_chip_screenshot`).
//!
//! ## Backend reality (Spec-Realism Gate / MT-008/015/020/022 pattern)
//!
//! AC-2 (create rich doc -> non-empty block_id parses as a LoomBlockAddr), AC-3 (self-seeded A -> B
//! inbound backlink), and AC-6 (save/refetch content_hash equals canonical SHA-256) are covered by the
//! unignored `live_pg_self_seeded_loom_block_backlink_hash_and_ui_proof` test behind the `integration`
//! feature. Run it against a live handshake_core on 127.0.0.1:37501; it creates its own documents,
//! uses fresh clients for read-back, and deletes the exact created ids even during panic. It NEVER
//! fakes PostgreSQL. The KERNEL_BUILDER gate established `content_hash` is
//! BACKEND-COMPUTED (no writable PATCH field on `LoomBlockUpdate`); AC-6 therefore READS the backend's
//! `content_hash` and asserts it equals the local canonical SHA-256 of the saved `content_json` — it
//! never client-PATCHes a hash.
//!
//! ## Artifact hygiene (CX-212E)
//!
//! EVERY PNG goes ONLY to the EXTERNAL `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-032/` root via
//! [`external_artifact_dir`]; [`assert_no_local_artifact_dir`] fails the run if a repo-local
//! `tests/screenshots/` or `test_output/` directory exists.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{By, NodeT, Queryable};
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::graph::canvas_board::{
    placement_author_id, CanvasPlacementCard, LoomCanvasBoard,
};
use handshake_native::interop::interaction_bus::{InteractionBus, CMD_OPEN_DOCUMENT};
use handshake_native::loom_address::{loom_uri, parse_loom_uri, ContentHash, LoomBlockAddr};
use handshake_native::loom_graph::{
    loom_node_author_id, GraphNode, LoomGraphColors, LoomGraphSurface,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::wikilinks::backlinks_panel::{
    dispatch_backlink_open, entry_author_id, render_backlinks_panel, PANEL_AUTHOR_ID,
};
use handshake_native::rich_editor::wikilinks::client::{
    ReqwestWikilinkBackend, RichDocBacklink, WikilinkBackend,
};
use handshake_native::rich_editor::wikilinks::runtime::{BacklinksState, WikilinkRuntime};
use handshake_native::theme::HsTheme;

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/` (the path a contract might literally name, overridden here).
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "CX-212E: no repo-local artifact dir may exist — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only (found {})",
            local.display()
        );
    }
}

fn live_editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());

    let registry = app.pane_registry();
    let mut guard = registry.lock().expect("registry");
    guard.insert(PaneRecord::new(
        PaneId::from("pane-a"),
        PaneType::CodeSymbol,
        DEFAULT_PROJECT_ID,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    guard.insert(PaneRecord::new(
        PaneId::from("pane-b"),
        PaneType::LoomWikiPage,
        DEFAULT_PROJECT_ID,
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    ));
    drop(guard);

    (app, runtime)
}

/// Serialize the `.wgpu()` screenshot tests (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// Collect every author_id present in the live AccessKit tree.
fn author_ids(harness: &Harness<'_, ()>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

/// Read a node's AccessKit `description` by author_id (the loom:// chip channel — AC-5).
fn description_for(harness: &Harness<'_, ()>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.description().map(|v| v.to_owned());
        }
    }
    None
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// PT-1 / AC-1: loom_uri / parse_loom_uri round-trip + ContentHash determinism (boundary re-assert).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn loom_uri_round_trip_at_boundary() {
    let addr = LoomBlockAddr::new("ws1", "block1");
    assert_eq!(loom_uri(&addr), "loom://ws1/block1");
    // AC-1: parse_loom_uri(loom_uri(&addr)) == Some(addr).
    assert_eq!(parse_loom_uri(&loom_uri(&addr)), Some(addr.clone()));
    // A UUID-shaped pair (the real backend id shape) also round-trips.
    let uuids = LoomBlockAddr::new(
        "11111111-1111-1111-1111-111111111111",
        "22222222-2222-2222-2222-222222222222",
    );
    assert_eq!(parse_loom_uri(&uuids.to_uri()), Some(uuids));
    // Malformed inputs reject (no fabricated address).
    assert_eq!(parse_loom_uri("https://ws/blk"), None);
    assert_eq!(parse_loom_uri("loom://ws"), None);
    println!("PT-1/AC-1: loom_uri/parse_loom_uri round-trips at the crate boundary");
}

#[test]
fn content_hash_deterministic_at_boundary() {
    // AC-6 (the pure half): the canonical hash is deterministic + key-order-independent.
    let a = serde_json::json!({ "b": 2, "a": 1, "nested": { "y": 1, "x": 2 } });
    let b = serde_json::json!({ "a": 1, "nested": { "x": 2, "y": 1 }, "b": 2 });
    let ha = ContentHash::of_content_json(&a);
    let hb = ContentHash::of_content_json(&b);
    assert_eq!(
        ha, hb,
        "structurally identical docs hash identically regardless of key order"
    );
    assert_eq!(ha.as_str().len(), 64);
    assert!(ha
        .as_str()
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    println!("PT-1/AC-6(pure): ContentHash is deterministic + canonical at the boundary");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 / PT-4: clicking a backlink row fires the shared-bus OpenDocument command with the right doc id.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// Build a headless [`WikilinkRuntime`] seeded with `backlinks` in the Loaded state for `document_id`
/// (no tokio runtime, no real fetch — the panel renders the seeded data directly).
fn seeded_backlinks_runtime(document_id: &str, backlinks: Vec<RichDocBacklink>) -> WikilinkRuntime {
    let backend: Arc<dyn WikilinkBackend> = Arc::new(ReqwestWikilinkBackend::production());
    let mut rt = WikilinkRuntime::new("ws-test", backend, None);
    rt.set_document(document_id);
    // Seed the Loaded state directly (the `backlinks` field is public; headless never re-fetches a
    // non-Idle state, so the panel renders these without a backend round-trip).
    rt.backlinks = BacklinksState::Loaded(backlinks);
    rt
}

fn backlink(src: &str, kind: &str) -> RichDocBacklink {
    RichDocBacklink {
        backlink_id: format!("BL-{src}"),
        workspace_id: "ws-test".into(),
        relationship_id: format!("REL-{src}"),
        source_document_id: src.into(),
        link_kind: kind.into(),
        target: "DOC-B".into(),
        block_id: format!("BLK-{src}"),
    }
}

fn one_shot_backlinks_server(
    status: &str,
    body: &str,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind backlinks capture server");
    let address = listener.local_addr().expect("capture server address");
    let status = status.to_owned();
    let body = body.to_owned();
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept backlinks request");
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .expect("bound capture read");
        let mut request = Vec::new();
        let mut chunk = [0_u8; 2048];
        loop {
            let count = stream.read(&mut chunk).expect("read backlinks request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&chunk[..count]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write backlinks response");
        String::from_utf8(request).expect("captured request is HTTP text")
    });
    (format!("http://{address}"), server)
}

fn recovering_backlinks_server() -> (String, std::thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind recovering backlinks server");
    let address = listener.local_addr().expect("recovering server address");
    let server = std::thread::spawn(move || {
        for (status, body) in [
            ("503 Service Unavailable", r#"{"error":"temporarily_down"}"#),
            (
                "200 OK",
                r#"{"source_document_id":"DOC-B","backlinks":[{"backlink_id":"BL-A","workspace_id":"ws-test","relationship_id":"REL-A","source_document_id":"DOC-A","link_kind":"wikilink","target":"DOC-B","block_id":"BLK-A"}]}"#,
            ),
        ] {
            let (mut stream, _) = listener.accept().expect("accept recovering request");
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .expect("bounded recovering read");
            let mut request = Vec::new();
            let mut chunk = [0_u8; 2048];
            loop {
                let count = stream.read(&mut chunk).expect("read recovering request");
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..count]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write recovering response");
        }
    });
    (format!("http://{address}"), server)
}

#[test]
fn loom_address_backlinks_transport_sends_required_identity_headers() {
    let (base_url, server) =
        one_shot_backlinks_server("200 OK", r#"{"source_document_id":"DOC-B","backlinks":[]}"#);
    let backend = ReqwestWikilinkBackend::new(base_url);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("backlinks transport runtime");
    let response = runtime
        .block_on(backend.list_backlinks("DOC-B"))
        .expect("identity-stamped backlinks request succeeds");
    assert_eq!(response.source_document_id, "DOC-B");
    assert!(response.backlinks.is_empty());

    let request = server.join().expect("join backlinks capture server");
    let request_lower = request.to_ascii_lowercase();
    assert!(request.starts_with("GET /knowledge/documents/DOC-B/backlinks HTTP/1.1\r\n"));
    assert!(request_lower.contains("x-hsk-actor-id: handshake-native-editor\r\n"));
    assert!(request_lower.contains("x-hsk-kernel-task-run-id: native-editor-backlinks-doc-b\r\n"));
    assert!(request_lower.contains("x-hsk-session-run-id: native-editor-wikilinks-"));
}

#[test]
fn loom_address_backlinks_404_is_empty_projection() {
    let (base_url, server) = one_shot_backlinks_server("404 Not Found", r#"{"error":"not_found"}"#);
    let backend = ReqwestWikilinkBackend::new(base_url);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("backlinks 404 runtime");
    let response = runtime
        .block_on(backend.list_backlinks("DOC-NEW"))
        .expect("404 is an empty backlink projection");
    assert_eq!(response.source_document_id, "DOC-NEW");
    assert!(response.backlinks.is_empty());
    let _ = server.join().expect("join backlinks 404 server");
}

#[test]
fn loom_address_backlinks_backend_down_is_bounded_failure() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("reserve unavailable backend address");
    let address = listener.local_addr().expect("unavailable address");
    drop(listener);
    let backend = ReqwestWikilinkBackend::new(format!("http://{address}"));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("backlinks failure runtime");
    let started = std::time::Instant::now();
    let result = runtime.block_on(backend.list_backlinks("DOC-DOWN"));
    assert!(
        matches!(
            result,
            Err(handshake_native::rich_editor::wikilinks::client::WikilinkError::NetworkError(_))
        ),
        "backend-down backlinks must surface a typed network error: {result:?}"
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(6),
        "backend-down backlinks call exceeded its five-second request bound"
    );
}

#[test]
fn loom_address_backlinks_refresh_recovers_failed_mounted_runtime() {
    let (base_url, server) = recovering_backlinks_server();
    let async_runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("backlinks recovery runtime");
    let backend: Arc<dyn WikilinkBackend> = Arc::new(ReqwestWikilinkBackend::new(base_url));
    let mut runtime =
        WikilinkRuntime::new("ws-test", backend, Some(async_runtime.handle().clone()));
    runtime.set_context("ws-test", "DOC-B");
    runtime.ensure_backlinks_loaded();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        runtime.drain();
        if matches!(runtime.backlinks, BacklinksState::Failed(_)) {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "initial failure timed out"
        );
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    runtime.refresh_backlinks();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        runtime.drain();
        match &runtime.backlinks {
            BacklinksState::Loaded(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].source_document_id, "DOC-A");
                break;
            }
            BacklinksState::Failed(err) => {
                panic!("refresh should recover the mounted runtime: {err}")
            }
            BacklinksState::Idle | BacklinksState::Loading => {}
        }
        assert!(std::time::Instant::now() < deadline, "recovery timed out");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    server.join().expect("join recovering backlinks server");
}

#[test]
fn backlink_click_fires_open_document() {
    // The shared bus the rich-text pane uses; register the cross-pane OpenDocument command (AC-4).
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    bus.lock().unwrap().register_open_document_command();

    // A document B with one inbound backlink from doc A — the MT-015 panel renders it.
    let runtime = Arc::new(Mutex::new(seeded_backlinks_runtime(
        "DOC-B",
        vec![backlink("DOC-A", "note")],
    )));

    let bus_ui = Arc::clone(&bus);
    let rt_ui = Arc::clone(&runtime);
    let ctx_for_click = Arc::new(Mutex::new(None::<egui::Context>));
    let ctx_capture = Arc::clone(&ctx_for_click);

    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 320.0))
        .build_ui(move |ui| {
            *ctx_capture.lock().unwrap() = Some(ui.ctx().clone());
            let pal = HsTheme::Dark.palette();
            let mut rt = rt_ui.lock().unwrap();
            if let Some(event) = render_backlinks_panel(ui, &mut rt, &pal) {
                // The melt-together bridge: a clicked backlink fires the shared-bus OpenDocument command.
                let mut bus = bus_ui.lock().unwrap();
                dispatch_backlink_open(ui.ctx(), &mut bus, &event);
            }
        });
    harness.run();

    // The backlink entry node is present (AC-7 shape).
    let ids = author_ids(&harness);
    let entry = entry_author_id("DOC-A");
    assert!(
        ids.contains(&entry),
        "AC-4: backlink entry '{entry}' present, got {ids:?}"
    );

    // No navigation staged before the click.
    assert!(
        bus.lock().unwrap().pending_navigation().is_none(),
        "nothing pending before click"
    );

    // Click the backlink entry by its Role::ListItem node carrying value "DOC-A (note)" — the panel
    // renders the clickable entry as a ListItem (its child TextRun shares the value, so disambiguate
    // by role).
    harness
        .get(
            By::new()
                .role(egui::accesskit::Role::ListItem)
                .value("DOC-A (note)"),
        )
        .click();
    harness.run();

    // AC-4: the click fired the OpenDocument command on the shared bus with the correct document id.
    let bus_guard = bus.lock().unwrap();
    assert!(
        bus_guard.commands().get(CMD_OPEN_DOCUMENT).is_some(),
        "the OpenDocument command is registered on the shared bus"
    );
    assert_eq!(
        bus_guard.pending_navigation(),
        Some("DOC-A"),
        "AC-4: clicking the backlink staged the source document id for a cross-pane open"
    );
    let _ = ctx_for_click; // (the click routed through the real ctx in build_ui)
    println!("AC-4/PT-4: backlink-row click fired OpenDocument on the shared bus for DOC-A");
}

#[test]
fn backlink_open_document_drains_through_live_shell() {
    let (app, _rt) = live_editor_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let bus = InteractionBus::get_or_init(&harness.ctx);
    let dispatched = InteractionBus::with_try_lock(&bus, |bus| {
        bus.register_open_document_command();
        bus.open_document(&harness.ctx, "DOC-LIVE-SHELL")
    })
    .unwrap_or(false);
    assert!(
        dispatched,
        "AC-4 live-shell: OpenDocument command dispatches on the app bus"
    );
    assert_eq!(
        InteractionBus::with_try_lock(&bus, |bus| bus.pending_navigation().map(str::to_owned))
            .flatten()
            .as_deref(),
        Some("DOC-LIVE-SHELL"),
        "AC-4 live-shell: backlink/open-document staged the document before the shell drain"
    );

    harness.run_steps(2);

    assert!(
        InteractionBus::with_try_lock(&bus, |bus| bus.pending_navigation().is_none())
            .unwrap_or(false),
        "AC-4 live-shell: HandshakeApp::drive_ckc_interop drained pending_navigation"
    );
    assert_eq!(
        harness.state().active_pane().map(|p| p.as_ref()),
        Some("pane-b"),
        "AC-4 live-shell: drained OpenDocument routed through ShellNavigator into the Notes pane"
    );
    assert!(
        harness.state().quick_switcher_nav_status().is_none(),
        "AC-4 live-shell: ShellNavigator OpenDocument landed on a real surface"
    );
    println!(
        "AC-4 live-shell: pending OpenDocument drained into ShellNavigator for DOC-LIVE-SHELL"
    );
}

#[test]
fn mounted_backlink_event_routes_through_bus_and_live_shell() {
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};
    use handshake_native::rich_editor::wikilinks::inline_view::EditorEvent;

    let (mut app, _rt) = live_editor_shell();
    let opened = app.open_document("DOC-BACKLINK-BEFORE");
    assert!(
        matches!(opened, NavDispatchOutcome::Opened { .. }),
        "precondition: open_document mounts the Notes editor; got {opened:?}"
    );
    let rich_state = app.mounted_rich_state();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::BacklinkActivated {
            source_document_id: "DOC-FROM-BACKLINK".to_owned(),
        });
    assert_eq!(
        rich_state.lock().unwrap().pending_events.len(),
        1,
        "precondition: mounted rich editor queued a BacklinkActivated event"
    );

    harness.run_steps(1);

    let bus = InteractionBus::get_or_init(&harness.ctx);
    assert_eq!(
        InteractionBus::with_try_lock(&bus, |bus| bus.pending_navigation().map(str::to_owned))
            .flatten()
            .as_deref(),
        Some("DOC-FROM-BACKLINK"),
        "AC-4 live-shell: mounted BacklinkActivated staged CMD_OPEN_DOCUMENT before shell drain"
    );
    let pane_b = PaneId::from("pane-b");
    let active_tab_before_drain = harness
        .state()
        .tab_bar_states()
        .get(&pane_b)
        .and_then(|bar| bar.active())
        .expect("pane-b active tab before drain");
    assert_eq!(
        active_tab_before_drain.content_id.as_deref(),
        Some("DOC-BACKLINK-BEFORE"),
        "AC-4 live-shell: direct ShellNavigator bypass did not open the backlink in the staging frame"
    );

    harness.run_steps(2);

    assert!(
        rich_state.lock().unwrap().pending_events.is_empty(),
        "AC-4 live-shell: mounted rich pending_events drained"
    );
    assert!(
        InteractionBus::with_try_lock(&bus, |bus| bus.pending_navigation().is_none())
            .unwrap_or(false),
        "AC-4 live-shell: CMD_OPEN_DOCUMENT pending_navigation drained by drive_ckc_interop"
    );
    assert_eq!(
        harness.state().active_pane(),
        Some(&pane_b),
        "AC-4 live-shell: backlink event opened/focused the mounted Notes pane"
    );
    let active_tab = harness
        .state()
        .tab_bar_states()
        .get(&pane_b)
        .and_then(|bar| bar.active())
        .expect("pane-b active tab");
    assert_eq!(
        active_tab.content_id.as_deref(),
        Some("DOC-FROM-BACKLINK"),
        "AC-4 live-shell: mounted BacklinkActivated routed through CMD_OPEN_DOCUMENT into ShellNavigator"
    );
    assert!(
        harness.state().quick_switcher_nav_status().is_none(),
        "AC-4 live-shell: routed backlink event landed without a typed nav error"
    );
    println!(
        "AC-4 live-shell: mounted BacklinkActivated -> dispatch_backlink_open -> CMD_OPEN_DOCUMENT -> ShellNavigator"
    );
}

#[test]
fn mounted_backlink_event_retries_when_bus_is_contended() {
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};
    use handshake_native::rich_editor::wikilinks::inline_view::EditorEvent;

    let (mut app, _rt) = live_editor_shell();
    let opened = app.open_document("DOC-BACKLINK-BEFORE");
    assert!(
        matches!(opened, NavDispatchOutcome::Opened { .. }),
        "precondition: open_document mounts the Notes editor; got {opened:?}"
    );
    let rich_state = app.mounted_rich_state();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    let bus = InteractionBus::get_or_init(&harness.ctx);
    let bus_guard = bus.lock().expect("hold bus for contention");
    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::BacklinkActivated {
            source_document_id: "DOC-RETRY-BACKLINK".to_owned(),
        });
    harness.run_steps(1);

    assert!(
        rich_state.lock().unwrap().pending_events.is_empty(),
        "retry proof: the mounted rich event queue drained even while the bus was contended"
    );
    assert!(
        harness
            .state()
            .quick_switcher_nav_status()
            .unwrap_or_default()
            .contains("retrying"),
        "retry proof: contended bus surfaces a retrying status, not a silent drop"
    );
    drop(bus_guard);

    harness.run_steps(1);
    assert_eq!(
        InteractionBus::with_try_lock(&bus, |bus| bus.pending_navigation().map(str::to_owned))
            .flatten()
            .as_deref(),
        Some("DOC-RETRY-BACKLINK"),
        "retry proof: once the bus is free, the queued backlink stages CMD_OPEN_DOCUMENT"
    );

    harness.run_steps(2);
    let pane_b = PaneId::from("pane-b");
    let active_tab = harness
        .state()
        .tab_bar_states()
        .get(&pane_b)
        .and_then(|bar| bar.active())
        .expect("pane-b active tab after retry drain");
    assert_eq!(
        active_tab.content_id.as_deref(),
        Some("DOC-RETRY-BACKLINK"),
        "retry proof: retried backlink opens through the live shell drain"
    );
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-7 / PT-5: the EXISTING MT-015 backlinks panel exposes backlinks-panel + at least one backlink-* node.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn backlinks_panel_accesskit_tree() {
    let runtime = Arc::new(Mutex::new(seeded_backlinks_runtime(
        "DOC-B",
        vec![backlink("DOC-A", "note"), backlink("DOC-C", "wp")],
    )));
    let rt_ui = Arc::clone(&runtime);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(420.0, 320.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let mut rt = rt_ui.lock().unwrap();
            let _ = render_backlinks_panel(ui, &mut rt, &pal);
        });
    harness.run();

    let ids = author_ids(&harness);
    // AC-7: the panel container node (the MT-015 panel author_id the contract reuses).
    assert!(
        ids.contains(PANEL_AUTHOR_ID),
        "AC-7: '{PANEL_AUTHOR_ID}' container present, got {ids:?}"
    );
    // AC-7: at least one backlink-{id} ListItem-equivalent node.
    let backlink_nodes = ids.iter().filter(|a| a.starts_with("backlink-")).count();
    assert!(
        backlink_nodes >= 1,
        "AC-7: at least one backlink-* node (got {backlink_nodes})"
    );
    assert!(
        ids.contains(&entry_author_id("DOC-A")),
        "AC-7: backlink-DOC-A present"
    );
    assert!(
        ids.contains(&entry_author_id("DOC-C")),
        "AC-7: backlink-DOC-C present"
    );
    println!("AC-7/PT-5: backlinks panel exposes '{PANEL_AUTHOR_ID}' + {backlink_nodes} backlink-* nodes");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-5 / AC-9: a canvas placement with placed_block_id exposes its loom:// chip as the AccessKit
// description; an empty placed_block_id has NO chip (RISK-3).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn board_with_cards(cards: Vec<CanvasPlacementCard>) -> Arc<Mutex<LoomCanvasBoard>> {
    let mut b = LoomCanvasBoard::new("ws-32", "canvas-1");
    b.set_board(cards, vec![], egui::Vec2::ZERO, 1.0);
    Arc::new(Mutex::new(b))
}

fn placed_card(placement_id: &str, block_id: &str, x: f32) -> CanvasPlacementCard {
    let mut c = CanvasPlacementCard::new(placement_id, block_id, x, 40.0, 220.0, 140.0);
    c.live_title = Some(format!("Title {placement_id}"));
    c.live_content_type = Some("note".to_owned());
    c
}

fn canvas_harness<'a>(board: Arc<Mutex<LoomCanvasBoard>>) -> Harness<'a, ()> {
    Harness::builder()
        .with_size(egui::vec2(900.0, 480.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = board.lock().unwrap().show(ui, &pal);
        })
}

#[test]
fn canvas_node_loom_chip_in_accesskit() {
    let board = board_with_cards(vec![placed_card("p-001", "blk-7", 30.0)]);
    let mut harness = canvas_harness(Arc::clone(&board));
    harness.run();

    let ids = author_ids(&harness);
    let node = placement_author_id("p-001");
    assert!(
        ids.contains(&node),
        "AC-5: placement node '{node}' present, got {ids:?}"
    );
    // AC-5: the placement's loom:// chip is the AccessKit description (ws-32 = board workspace).
    let desc = description_for(&harness, &node);
    assert_eq!(
        desc.as_deref(),
        Some("loom://ws-32/blk-7"),
        "AC-5: canvas node exposes its loom:// chip in the AccessKit description"
    );
    println!(
        "AC-5/AC-9: canvas placement p-001 exposes loom://ws-32/blk-7 in its AccessKit description"
    );
}

#[test]
fn canvas_node_loom_chip_includes_content_hash_suffix() {
    // When the host has resolved a backend content_hash, the chip carries a short ` #<8hex>` suffix
    // (READ-only — the canvas never writes a hash).
    let mut card = placed_card("p-009", "blk-9", 30.0);
    card.loom_content_hash =
        Some("44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned());
    let board = board_with_cards(vec![card]);
    let mut harness = canvas_harness(Arc::clone(&board));
    harness.run();

    let desc = description_for(&harness, &placement_author_id("p-009"));
    assert_eq!(
        desc.as_deref(),
        Some("loom://ws-32/blk-9 #44136fa3"),
        "AC-5: the chip carries the short content-hash suffix when resolved"
    );
    println!("AC-5: resolved content_hash adds a short ' #44136fa3' suffix to the loom:// chip");
}

#[test]
fn empty_placement_has_no_loom_chip() {
    // RISK-3: a placement with an empty placed_block_id renders NO chip — its node has no description,
    // no panic, no fabricated loom:// URI.
    let board = board_with_cards(vec![placed_card("p-empty", "", 30.0)]);
    let mut harness = canvas_harness(Arc::clone(&board));
    harness.run();

    let node = placement_author_id("p-empty");
    let desc = description_for(&harness, &node);
    assert_eq!(
        desc, None,
        "RISK-3: an empty placed_block_id has no loom:// chip description"
    );
    println!("RISK-3: empty placed_block_id => no loom:// chip (no panic, no fabricated URI)");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// PT-5 (graph): a graph node tooltip exposes its loom:// URI + backlink count via the AccessKit description.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn graph_node_loom_tooltip_in_accesskit() {
    use handshake_native::context_menu_surfaces::LoomNodeState;

    let node = GraphNode::new(
        LoomNodeState {
            block_id: "blk-1".into(),
            pinned: false,
            favorite: false,
            has_edges: true,
        },
        "Graph Note",
    )
    .with_backlink_count(2);
    let surface = LoomGraphSurface::with_workspace(vec![node], "ws-32");

    let surface_ui = surface.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 240.0))
        .build_ui(move |ui| {
            let colors = LoomGraphColors {
                node_bg: egui::Color32::from_gray(40),
                node_hover_bg: egui::Color32::from_gray(60),
                node_text: egui::Color32::WHITE,
            };
            let _ = surface_ui.show(ui, colors);
        });
    harness.run();

    let author = loom_node_author_id("blk-1");
    let ids = author_ids(&harness);
    assert!(
        ids.contains(&author),
        "graph node '{author}' present, got {ids:?}"
    );
    let desc = description_for(&harness, &author);
    assert_eq!(
        desc.as_deref(),
        Some("loom://ws-32/blk-1; 2 backlinks"),
        "PT-5: graph node tooltip exposes its loom:// URI + backlink count in the AccessKit description"
    );
    println!("PT-5: graph node blk-1 exposes 'loom://ws-32/blk-1; 2 backlinks' in its AccessKit description");
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// HBR-VIS: a .wgpu() screenshot of the canvas with a loom:// chip card (EXTERNAL artifact root only).
// ═════════════════════════════════════════════════════════════════════════════════════════════════

fn save_canvas_loom_screenshot(board: Arc<Mutex<LoomCanvasBoard>>, filename: &str) -> PathBuf {
    let board_ui = Arc::clone(&board);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 480.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            let _ = board_ui.lock().unwrap().show(ui, &pal);
        });
    harness.run();
    harness.run();
    let image = harness
        .render()
        .expect("HBR-VIS: the Canvas loom-address surface must render through wgpu");
    assert!(
        image.width() > 0 && image.height() > 0,
        "rendered Canvas image must be non-empty"
    );
    let ext_dir = external_artifact_dir("wp-kernel-012-mt-032");
    std::fs::create_dir_all(&ext_dir).expect("create external MT-032 artifact directory");
    let png = ext_dir.join(filename);
    if png.exists() {
        std::fs::remove_file(&png)
            .unwrap_or_else(|err| panic!("remove stale Canvas visual {}: {err}", png.display()));
    }
    image
        .save(&png)
        .unwrap_or_else(|err| panic!("save strict Canvas visual {}: {err}", png.display()));
    let bytes = std::fs::metadata(&png)
        .unwrap_or_else(|err| panic!("stat strict Canvas visual {}: {err}", png.display()))
        .len();
    assert!(
        bytes > 0,
        "strict Canvas visual is empty: {}",
        png.display()
    );
    let decoded = image::open(&png)
        .unwrap_or_else(|err| panic!("reopen strict Canvas visual {}: {err}", png.display()));
    assert_eq!(decoded.width(), image.width());
    assert_eq!(decoded.height(), image.height());
    println!(
        "HBR-VIS: {}x{} strict Canvas loom-address screenshot saved={} bytes={bytes}",
        image.width(),
        image.height(),
        png.display()
    );
    png
}

#[test]
fn canvas_loom_chip_screenshot() {
    let _g = wgpu_guard();
    let board = board_with_cards(vec![
        placed_card("p-001", "blk-7", 40.0),
        placed_card("p-002", "blk-8", 320.0),
    ]);
    let _ = save_canvas_loom_screenshot(board, "MT-032-canvas-loom-chips.png");
    assert_no_local_artifact_dir();
}

// ═════════════════════════════════════════════════════════════════════════════════════════════════
// AC-2 / AC-3 / AC-6: LIVE-PG integration (NEEDS_MANAGED_RESOURCE_PROOF — `--features integration`).
//
// These require a running handshake_core on 127.0.0.1:37501 with a real workspace. They NEVER fake PG.
// content_hash is BACKEND-COMPUTED (KERNEL_BUILDER gate): AC-6 READS the backend's content_hash and
// asserts it equals the local canonical SHA-256 of the saved content_json — no client PATCH of a hash.
// ═════════════════════════════════════════════════════════════════════════════════════════════════

/// The real backend base URL the integration tests talk to.
#[cfg(feature = "integration")]
const LIVE_BASE_URL: &str = "http://127.0.0.1:37501";

/// Attach the three mandatory rich-document context headers the backend's `doc_context(&headers)`
/// requires (`x-hsk-actor-id` / `x-hsk-kernel-task-run-id` / `x-hsk-session-run-id`). The real
/// `create_document` (POST /knowledge/documents) and `list_backlinks`
/// (GET /knowledge/documents/{id}/backlinks) handlers return HTTP 400 when any is absent; the React
/// reference sends them via `richDocHeaders(ctx)` (api.ts), with `operator` as the default actor id.
/// `getLoomBlock` (GET /workspaces/{ws}/loom/blocks/{id}) correctly needs NONE, so those calls stay
/// header-free.
#[cfg(feature = "integration")]
fn with_rich_doc_headers(rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    rb.header("x-hsk-actor-id", "operator")
        .header("x-hsk-actor-kind", "operator")
        .header("x-hsk-kernel-task-run-id", "KTR-EDITOR-UI")
        .header("x-hsk-session-run-id", "MT-032-integration")
}

#[cfg(feature = "integration")]
fn required_doc_field(doc: &serde_json::Value, field: &str) -> String {
    doc.get(field)
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| panic!("live PG document response missing required '{field}': {doc}"))
        .to_owned()
}

#[cfg(feature = "integration")]
fn rich_document_id(doc: &serde_json::Value) -> String {
    required_doc_field(doc, "rich_document_id")
}

#[cfg(feature = "integration")]
fn optional_loom_block_id(doc: &serde_json::Value) -> Option<&str> {
    doc.get("block_id")
        .or_else(|| doc.get("loom_block_id"))
        .and_then(|x| x.as_str())
        .filter(|s| !s.trim().is_empty())
}

#[cfg(feature = "integration")]
fn require_loom_block_id(doc: &serde_json::Value, ac: &str, rich_document_id: &str) -> String {
    optional_loom_block_id(doc)
        .unwrap_or_else(|| {
            panic!(
                "{ac}: BACKEND_SHAPE_GAP_NO_RICH_DOC_LOOM_BLOCK_ID: created rich document \
                 {rich_document_id} has no block_id/loom_block_id in the live backend response; \
                 do not mark MT-032 live Loom-block addressability proven from this response"
            )
        })
        .to_owned()
}

#[cfg(feature = "integration")]
fn live_client() -> reqwest::Client {
    reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("bounded live HTTP client")
}

/// Tracks the exact authority ids created by the live proof. `delete_all` is
/// the normal deterministic path; Drop repeats it on a private one-thread
/// runtime if an assertion unwinds before cleanup.
#[cfg(feature = "integration")]
struct LiveDocumentCleanup {
    ids: Arc<Mutex<Vec<String>>>,
}

#[cfg(feature = "integration")]
impl LiveDocumentCleanup {
    fn new() -> Self {
        Self {
            ids: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn track(&self, document_id: String) {
        self.ids.lock().unwrap().push(document_id);
    }

    fn untrack(&self, document_id: &str) {
        self.ids.lock().unwrap().retain(|id| id != document_id);
    }

    async fn delete_all(&self) -> Result<(), String> {
        let ids = self.ids.lock().unwrap().clone();
        let client = live_client();
        for document_id in ids.iter().rev() {
            let response = with_rich_doc_headers(
                client.delete(format!("{LIVE_BASE_URL}/knowledge/documents/{document_id}")),
            )
            .send()
            .await
            .map_err(|err| format!("cleanup DELETE {document_id}: {err}"))?;
            if !response.status().is_success() {
                return Err(format!(
                    "cleanup DELETE {document_id}: HTTP {}",
                    response.status()
                ));
            }
        }
        self.ids.lock().unwrap().clear();
        Ok(())
    }
}

#[cfg(feature = "integration")]
async fn save_rich_document(
    client: &reqwest::Client,
    document_id: &str,
    expected_version: u64,
    content_json: serde_json::Value,
) -> serde_json::Value {
    let response = with_rich_doc_headers(client.put(format!(
        "{LIVE_BASE_URL}/knowledge/documents/{document_id}/save"
    )))
    .json(&serde_json::json!({
        "expected_version": expected_version,
        "content_json": content_json,
    }))
    .send()
    .await
    .unwrap_or_else(|err| panic!("save {document_id}: {err}"));
    assert_eq!(
        response.status().as_u16(),
        200,
        "save {document_id} must succeed"
    );
    response
        .json()
        .await
        .unwrap_or_else(|err| panic!("save {document_id} JSON: {err}"))
}

#[cfg(feature = "integration")]
async fn load_backlinks_runtime(
    handle: tokio::runtime::Handle,
    workspace_id: &str,
    document_id: &str,
) -> Arc<Mutex<WikilinkRuntime>> {
    let backend: Arc<dyn WikilinkBackend> = Arc::new(ReqwestWikilinkBackend::new(LIVE_BASE_URL));
    let mut runtime = WikilinkRuntime::new(workspace_id, backend, Some(handle));
    runtime.set_context(workspace_id, document_id);
    runtime.ensure_backlinks_loaded();
    let runtime = Arc::new(Mutex::new(runtime));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(8);
    loop {
        let state = {
            let mut runtime_guard = runtime.lock().expect("live backlinks runtime");
            runtime_guard.drain();
            runtime_guard.backlinks.clone()
        };
        match state {
            BacklinksState::Loaded(_) => return runtime,
            BacklinksState::Failed(err) => {
                panic!("production backlinks runtime failed for {document_id}: {err}")
            }
            BacklinksState::Idle | BacklinksState::Loading => {}
        }
        assert!(
            std::time::Instant::now() < deadline,
            "production backlinks runtime timed out for {document_id}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
}

#[cfg(feature = "integration")]
fn loaded_backlinks(runtime: &Arc<Mutex<WikilinkRuntime>>) -> Vec<RichDocBacklink> {
    match &runtime.lock().expect("live backlinks runtime").backlinks {
        BacklinksState::Loaded(rows) => rows.clone(),
        state => panic!("expected loaded backlinks, got {state:?}"),
    }
}

#[cfg(feature = "integration")]
impl Drop for LiveDocumentCleanup {
    fn drop(&mut self) {
        let ids = self.ids.lock().unwrap().clone();
        if ids.is_empty() {
            return;
        }
        let _ = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("cleanup runtime");
            runtime.block_on(async move {
                let client = live_client();
                for document_id in ids.iter().rev() {
                    let _ = with_rich_doc_headers(
                        client.delete(format!("{LIVE_BASE_URL}/knowledge/documents/{document_id}")),
                    )
                    .send()
                    .await;
                }
            });
        })
        .join();
    }
}

/// AC-2/3/4/5/6/7: one self-seeded, managed-resource proof. It creates B and A, saves A -> B through
/// the canonical save path (never the repair endpoint), loads B through the production wikilink
/// transport/runtime, clicks the real row, proves removal/re-add/delete reactivity, renders B's live
/// loom:// address, and refetches authority + LoomBlock hashes through fresh clients.
#[test]
#[cfg(feature = "integration")]
fn live_pg_self_seeded_loom_block_backlink_hash_and_ui_proof() {
    let rt = tokio::runtime::Runtime::new().expect("integration runtime");
    let runtime_handle = rt.handle().clone();
    rt.block_on(async {
        let seed_client = live_client();
        let workspace_id = live_workspace_id(&seed_client).await.expect("a live workspace");
        let cleanup = LiveDocumentCleanup::new();
        let run_suffix = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos()
        );

        let doc_b = create_rich_document(
            &seed_client,
            &workspace_id,
            &format!("MT-032-{run_suffix}-B"),
            serde_json::json!({
                "type": "doc",
                "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "target B" }] }]
            }),
        )
        .await
        .expect("create B");
        let b_id = rich_document_id(&doc_b);
        cleanup.track(b_id.clone());
        let b_block_id = require_loom_block_id(&doc_b, "AC-2 B", &b_id);
        assert_eq!(
            b_block_id, b_id,
            "AC-2: the canonical RichDocument and first-class LoomBlock share identity"
        );
        let b_addr = LoomBlockAddr::new(&workspace_id, &b_block_id);
        assert!(b_addr.is_addressable(), "AC-2: B has an addressable LoomBlock");
        assert_eq!(parse_loom_uri(&b_addr.to_uri()), Some(b_addr.clone()));

        // A fresh authority load must preserve the same canonical block identity.
        let fresh_b_response = with_rich_doc_headers(seed_client.get(format!(
            "{LIVE_BASE_URL}/knowledge/documents/{b_id}"
        )))
        .send()
        .await
        .expect("fresh load B");
        assert_eq!(fresh_b_response.status().as_u16(), 200);
        let fresh_b: serde_json::Value = fresh_b_response.json().await.expect("fresh B JSON");
        let fresh_b_doc = &fresh_b["document"];
        assert_eq!(rich_document_id(fresh_b_doc), b_id);
        assert_eq!(
            require_loom_block_id(fresh_b_doc, "AC-2 fresh B", &b_id),
            b_block_id
        );

        let empty_a_content = serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "source A" }] }]
        });
        let doc_a = create_rich_document(
            &seed_client,
            &workspace_id,
            &format!("MT-032-{run_suffix}-A"),
            empty_a_content.clone(),
        )
        .await
        .expect("create A without a link");
        let a_id = rich_document_id(&doc_a);
        cleanup.track(a_id.clone());
        let a_block_id = require_loom_block_id(&doc_a, "AC-2 A", &a_id);
        assert_eq!(a_block_id, a_id);
        assert_eq!(
            parse_loom_uri(&loom_uri(&LoomBlockAddr::new(&workspace_id, &a_block_id))),
            Some(LoomBlockAddr::new(&workspace_id, &a_block_id))
        );

        // Production mutation path: create the A -> B edge by SAVE. No backlink repair/rebuild call
        // exists anywhere in this proof.
        let linked_a_content = serde_json::json!({
            "type": "doc",
            "content": [{
                "type": "paragraph",
                "content": [
                    { "type": "text", "text": "see " },
                    { "type": "hsLink", "attrs": { "refKind": "note", "refValue": b_id, "label": "target B" } }
                ]
            }]
        });
        let link_save = save_rich_document(&seed_client, &a_id, 1, linked_a_content.clone()).await;
        assert!(
            link_save["backlinks_persisted"].as_u64().unwrap_or(0) >= 1,
            "save-time indexing must persist A -> B: {link_save}"
        );
        assert!(link_save["backlinks_error"].is_null(), "{link_save}");
        assert!(link_save["backlinks_skipped_reason"].is_null(), "{link_save}");

        // End-to-end production client/runtime: Idle -> Loading -> Loaded against managed PG.
        let panel_runtime = load_backlinks_runtime(
            runtime_handle.clone(),
            &workspace_id,
            &b_id,
        )
        .await;
        let inbound = loaded_backlinks(&panel_runtime);
        assert!(
            inbound.iter().any(|row| {
                row.source_document_id == a_id && row.link_kind == "wikilink" && row.target == b_id
            }),
            "AC-3: B's inbound backlinks contain source A"
        );

        // Mount the ACTUAL live-loaded runtime and click its exact AccessKit ListItem.
        let bus = Arc::new(Mutex::new(InteractionBus::new()));
        bus.lock().unwrap().register_open_document_command();
        let bus_ui = Arc::clone(&bus);
        let runtime_ui = Arc::clone(&panel_runtime);
        let mut panel = Harness::builder()
            .with_size(egui::vec2(520.0, 360.0))
            .build_ui(move |ui| {
                let mut runtime = runtime_ui.lock().unwrap();
                if let Some(event) = render_backlinks_panel(ui, &mut runtime, &HsTheme::Dark.palette()) {
                    dispatch_backlink_open(ui.ctx(), &mut bus_ui.lock().unwrap(), &event);
                }
            });
        panel.run();
        let ids = author_ids(&panel);
        assert!(ids.contains(PANEL_AUTHOR_ID));
        let row_author_id = entry_author_id(&a_id);
        assert!(ids.contains(&row_author_id));
        let mut saw_list = false;
        let mut saw_clickable_list_item = false;
        for node in panel.root().children_recursive() {
            let ak = node.accesskit_node();
            if ak.author_id() == Some(PANEL_AUTHOR_ID) {
                assert_eq!(ak.role(), egui::accesskit::Role::List);
                saw_list = true;
            }
            if ak.author_id() == Some(row_author_id.as_str()) {
                assert_eq!(ak.role(), egui::accesskit::Role::ListItem);
                assert!(ak.data().supports_action(egui::accesskit::Action::Click));
                saw_clickable_list_item = true;
            }
        }
        assert!(saw_list && saw_clickable_list_item);
        let row_value = format!("{a_id} (wikilink)");
        panel
            .get(
                By::new()
                    .role(egui::accesskit::Role::ListItem)
                    .value(&row_value),
            )
            .click();
        panel.run();
        assert_eq!(bus.lock().unwrap().pending_navigation(), Some(a_id.as_str()));
        assert!(bus
            .lock()
            .unwrap()
            .commands()
            .get(CMD_OPEN_DOCUMENT)
            .is_some());

        // Render the current LIVE B address on the real canvas card and inspect AccessKit.
        let canvas = board_with_cards(vec![placed_card("live-B", &b_block_id, 30.0)]);
        canvas.lock().unwrap().workspace_id = workspace_id.clone();
        let mut canvas_harness = canvas_harness(Arc::clone(&canvas));
        canvas_harness.run();
        let canvas_description = description_for(&canvas_harness, &placement_author_id("live-B"))
            .expect("live canvas placement AccessKit description");
        assert!(canvas_description.contains(&b_addr.to_uri()));
        drop(canvas_harness);
        {
            let _guard = wgpu_guard();
            let _ = save_canvas_loom_screenshot(
                Arc::clone(&canvas),
                "MT-032-canvas-live-B.png",
            );
        }

        // Remove then restore A -> B via successive canonical saves. Each fresh production runtime
        // must observe the new projection; no cached or seeded rows are accepted.
        let remove_save = save_rich_document(&seed_client, &a_id, 2, empty_a_content.clone()).await;
        assert_eq!(remove_save["backlinks_persisted"].as_u64(), Some(0));
        assert!(remove_save["backlinks_error"].is_null(), "{remove_save}");
        assert!(remove_save["backlinks_skipped_reason"].is_null(), "{remove_save}");
        let after_remove = load_backlinks_runtime(
            runtime_handle.clone(),
            &workspace_id,
            &b_id,
        )
        .await;
        assert!(loaded_backlinks(&after_remove).is_empty());

        let restore_save =
            save_rich_document(&seed_client, &a_id, 3, linked_a_content.clone()).await;
        assert!(restore_save["backlinks_persisted"].as_u64().unwrap_or(0) >= 1);
        assert!(restore_save["backlinks_error"].is_null(), "{restore_save}");
        assert!(restore_save["backlinks_skipped_reason"].is_null(), "{restore_save}");
        let after_restore = load_backlinks_runtime(
            runtime_handle.clone(),
            &workspace_id,
            &b_id,
        )
        .await;
        assert!(loaded_backlinks(&after_restore)
            .iter()
            .any(|row| row.source_document_id == a_id));

        // Save B to a new canonical body. A FRESH client refetches the document and LoomBlock.
        let saved_content = serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": format!("saved-{run_suffix}") }] }]
        });
        let expected_hash = ContentHash::of_content_json(&saved_content);
        let save_b = save_rich_document(&seed_client, &b_id, 1, saved_content.clone()).await;
        assert!(save_b["backlinks_error"].is_null(), "{save_b}");
        assert!(save_b["backlinks_skipped_reason"].is_null(), "{save_b}");

        let refetch_client = live_client();
        let document_response = with_rich_doc_headers(refetch_client.get(format!(
            "{LIVE_BASE_URL}/knowledge/documents/{b_id}"
        )))
        .send()
        .await
        .expect("refetch B document");
        assert_eq!(document_response.status().as_u16(), 200);
        let document_body: serde_json::Value = document_response.json().await.expect("B document body");
        let reloaded = &document_body["document"];
        assert_eq!(reloaded["content_json"], saved_content);
        assert_eq!(reloaded["content_sha256"].as_str(), Some(expected_hash.as_str()));

        let block_response = refetch_client
            .get(format!(
                "{LIVE_BASE_URL}/workspaces/{workspace_id}/loom/blocks/{b_block_id}"
            ))
            .send()
            .await
            .expect("refetch B LoomBlock");
        assert_eq!(block_response.status().as_u16(), 200);
        let block_body: serde_json::Value = block_response.json().await.expect("B block body");
        assert_eq!(block_body["block_id"].as_str(), Some(b_block_id.as_str()));
        assert_eq!(block_body["workspace_id"].as_str(), Some(workspace_id.as_str()));
        assert_eq!(block_body["content_hash"].as_str(), Some(expected_hash.as_str()));

        // Delete A and prove authority, Loom projection, and B's inbound projection all clean up.
        let delete_a = with_rich_doc_headers(seed_client.delete(format!(
            "{LIVE_BASE_URL}/knowledge/documents/{a_id}"
        )))
        .send()
        .await
        .expect("delete A");
        assert_eq!(delete_a.status().as_u16(), 200);
        let delete_a_body: serde_json::Value = delete_a.json().await.expect("delete A JSON");
        assert_eq!(delete_a_body["deleted"].as_bool(), Some(true));
        assert_eq!(delete_a_body["loom_block_deleted"].as_bool(), Some(true));
        assert!(delete_a_body["backlinks_deleted"].as_u64().unwrap_or(0) >= 1);
        cleanup.untrack(&a_id);

        let deleted_document = with_rich_doc_headers(seed_client.get(format!(
            "{LIVE_BASE_URL}/knowledge/documents/{a_id}"
        )))
        .send()
        .await
        .expect("refetch deleted A");
        assert_eq!(deleted_document.status().as_u16(), 404);
        let deleted_block = seed_client
            .get(format!(
                "{LIVE_BASE_URL}/workspaces/{workspace_id}/loom/blocks/{a_block_id}"
            ))
            .send()
            .await
            .expect("refetch deleted A LoomBlock");
        assert_eq!(deleted_block.status().as_u16(), 404);
        let after_delete = load_backlinks_runtime(
            runtime_handle.clone(),
            &workspace_id,
            &b_id,
        )
        .await;
        assert!(loaded_backlinks(&after_delete).is_empty());

        println!(
            "MT-032 LIVE PROOF workspace_id={workspace_id} A_document_id={a_id} A_block_id={a_block_id} B_document_id={b_id} B_block_id={b_block_id} content_hash={} inbound_rows={} remove_restore_delete=pass",
            expected_hash.as_str(),
            inbound.len()
        );
        cleanup.delete_all().await.expect("delete exact MT-032 fixture ids");
    });
}

/// Resolve a live workspace id by listing workspaces (the first one). Returns `None` when the backend is
/// unreachable / empty (the integration test is `#[ignore]` so this only runs against a seeded backend).
#[cfg(feature = "integration")]
async fn live_workspace_id(client: &reqwest::Client) -> Option<String> {
    let resp = client
        .get(format!("{LIVE_BASE_URL}/workspaces"))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let v: serde_json::Value = resp.json().await.ok()?;
    v.as_array()
        .and_then(|a| a.first())
        .and_then(|w| w.get("workspace_id").or_else(|| w.get("id")))
        .and_then(|x| x.as_str())
        .map(ToOwned::to_owned)
}

/// Create a rich document via the live backend knowledge-document API, returning the response JSON
/// (carrying `document_id` + `block_id`).
#[cfg(feature = "integration")]
async fn create_rich_document(
    client: &reqwest::Client,
    workspace_id: &str,
    title: &str,
    content_json: serde_json::Value,
) -> Option<serde_json::Value> {
    let url = format!("{LIVE_BASE_URL}/knowledge/documents");
    let body = serde_json::json!({
        "workspace_id": workspace_id,
        "title": title,
        "content_json": content_json,
    });
    let resp = with_rich_doc_headers(client.post(&url).json(&body))
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    resp.json::<serde_json::Value>()
        .await
        .ok()?
        .get("document")
        .cloned()
}
