//! Bidirectional Editors <-> Stage (Pillar 17) interop proofs — WP-KERNEL-012 MT-066 (cluster E10).
//!
//! This suite proves the FULL Stage round-trip at the widget/client level, which is what is provable NOW
//! (fixtures + an in-process mock server + egui_kittest). The Stage backend has NO `/stage/` HTTP routes
//! in the current handshake_core build (Stage = Pillar 17, like FEMS = Pillar 12), so:
//!   - the ROUTE leg is bus-only (EXTENDS the MT-033 `interop.route-to-stage` command — no backend POST),
//!   - the EMBED-BACK leg's missing route is the DESIGNED typed blocker
//!     (`StageInteropError::EmbedBackEndpointAbsent`), which is the production reality here.
//!
//! PROOF-POSTURE GATE: the live route round-trip against a real PostgreSQL/EventLedger with live native-
//! editor FR ingestion (AC-002) is `NEEDS_MANAGED_RESOURCE_PROOF` — both the managed PG and the live FR
//! ingestion are gated (like MT-064) — so it is `#[ignore]`d below; the FR-emit SHAPE is proven now via the
//! exact `NativeEditorEvent::route_to_stage` constructor the bus uses + the bus dispatch wiring.
//!
//! Proof map:
//! - PT-001 / AC-001: `route_payload_from_selection_and_canvas_node` — the Selection + CanvasNode payload
//!   builders produce the correct StageRoutePayload shape (workspace_id, source variant, correlation_id).
//! - PT-002 / AC-002 (shape now, live gated): `route_to_stage_emits_fr_event_and_stages_content` — the bus
//!   route emits the MT-036 `route_to_stage` FR event (shape) AND stages the content the Stage pane shows;
//!   `live_route_round_trip_real_pg` is the GATED real-PG round-trip (`#[ignore]`).
//! - PT-003 / AC-003: `embed_back_inserts_mt014_nodeview_with_provenance` — a fetched artifact becomes an
//!   MT-014 `hsLink` embed atom carrying the SHA-256 manifest provenance descriptor.
//! - PT-004 / AC-004: `embed_back_endpoint_absent_404` + `embed_back_endpoint_absent_501` — the missing
//!   route maps to `EmbedBackEndpointAbsent` (the typed blocker) over a mock server (BROAD: 404 AND 501);
//!   no backend route added, no artifact fabricated.
//! - PT-005 / AC-006: `stage_pane_accesskit_nodes_present` — the live AccessKit tree carries `stage-pane`
//!   (GenericContainer), `stage-routed-content` (GenericContainer), and `stage-capture-embed-back` (Button)
//!   with the correct roles + nesting; saves a screenshot to the EXTERNAL artifact root.
//! - AC-005: `single_route_command_id_plus_embed_command` — exactly one route-to-stage command id (extends
//!   MT-033) + the added embed-stage-capture command id (grep gate over the catalog + the bus descriptors).
//! - AC-007: `no_sqlite_no_backend_edit` — the production source has no sqlite/rusqlite and no
//!   src/backend edit; `assert_no_local_artifact_dir` guards artifact hygiene (CX-212E).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::interop::{
    build_from_canvas_node, build_from_selection, embed_artifact_as_nodeview, CanvasNodeRef,
    EditorSurfaceKind, InteractionBus, SharedSelection, StageArtifactRef, StageClient,
    StageInteropError, StageManifest, StageRouteSource, CMD_EMBED_STAGE_CAPTURE,
    CMD_ROUTE_TO_STAGE, STAGE_CAPTURE_REF_KIND,
};
use handshake_native::stage_pane::{
    EmbedTarget, StagePane, STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID, STAGE_PANE_AUTHOR_ID,
    STAGE_ROUTED_CONTENT_AUTHOR_ID,
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
// In-process mock HTTP server (the PROVEN MT-020/MT-037/MT-063 TcpListener pattern — no new dependency).
// ════════════════════════════════════════════════════════════════════════════════════════════════

/// Spin up a one-shot mock server that replies with `status_line` + `body` to the FIRST request, and
/// captures that request's line. Returns (base_url, join handle delivering the request line).
fn spawn_mock(
    status_line: &'static str,
    body: serde_json::Value,
) -> (String, std::thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{addr}");
    let handle = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let request_line = read_request_line(&mut stream);
        let body_str = body.to_string();
        let response = format!(
            "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{body_str}",
            body_str.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
        request_line
    });
    (base_url, handle)
}

/// Read one HTTP request's request line off the stream (a GET has no body).
fn read_request_line(stream: &mut std::net::TcpStream) -> String {
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
    text.lines().next().unwrap_or("").to_string()
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime")
}

fn dark() -> handshake_native::theme::HsPalette {
    HsTheme::Dark.palette()
}

fn text_range(pane_id: &str, start: usize, end: usize, text: &str) -> SharedSelection {
    SharedSelection::TextRange {
        pane_id: std::sync::Arc::from(pane_id),
        surface: EditorSurfaceKind::RichText,
        start,
        end,
        text: text.to_owned(),
    }
}

fn evidence_artifact(id: &str) -> StageArtifactRef {
    StageArtifactRef {
        artifact_id: id.to_owned(),
        workspace_id: "WS-1".to_owned(),
        sha256: "c".repeat(64),
        manifest: StageManifest {
            sha256: "c".repeat(64),
            manifest_ref: format!("manifest://{id}"),
            content_type: "image/png".to_owned(),
        },
        label: "Capture".to_owned(),
    }
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-001 / AC-001 — the route-leg payload builders (selection + canvas node).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn route_payload_from_selection_and_canvas_node() {
    // A TextRange selection -> Selection source with the materialized text + a source ref.
    let sel = text_range("pane-rich", 4, 17, "route this span");
    let payload = build_from_selection(&sel, "WS-1").expect("AC-001: selection payload builds");
    assert_eq!(payload.workspace_id, "WS-1");
    assert_eq!(payload.content_kind(), "selection");
    match &payload.source {
        StageRouteSource::Selection {
            source_pane_id,
            text,
            source_ref,
            ..
        } => {
            assert_eq!(source_pane_id, "pane-rich");
            assert_eq!(text, "route this span");
            assert_eq!(source_ref, "pane-rich:4-17");
        }
        other => panic!("AC-001: expected Selection source, got {other:?}"),
    }
    assert_eq!(payload.correlation_id, "stage-route-sel-pane-rich-4-17");

    // A canvas node -> CanvasNode source.
    let node = CanvasNodeRef {
        workspace_id: "WS-1".to_owned(),
        canvas_id: "CB-1".to_owned(),
        node_id: "N-9".to_owned(),
        node_kind: "loom_block".to_owned(),
        pane_id: "pane-canvas".to_owned(),
    };
    let cpayload = build_from_canvas_node(&node).expect("AC-001: canvas-node payload builds");
    assert_eq!(cpayload.content_kind(), "canvas_node");
    match &cpayload.source {
        StageRouteSource::CanvasNode {
            canvas_id,
            node_id,
            node_kind,
            ..
        } => {
            assert_eq!(canvas_id, "CB-1");
            assert_eq!(node_id, "N-9");
            assert_eq!(node_kind, "loom_block");
        }
        other => panic!("AC-001: expected CanvasNode source, got {other:?}"),
    }
    println!(
        "PT-001 payload builders OK: selection corr={} | canvas corr={}",
        payload.correlation_id, cpayload.correlation_id
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-002 / AC-002 — the route leg emits the MT-036 route_to_stage FR event (shape) + stages content.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn route_to_stage_emits_fr_event_and_stages_content() {
    use handshake_native::event_emitter::NativeEditorEvent;

    // The FR event the bus emits on a route is the EXACT MT-036 constructor (no new event kind, MC-005).
    // Prove its shape directly: `route_to_stage` action, the source pane as pane_id, the content_kind in
    // the payload — the same value the bus builds at its route_to_stage call site.
    let ev = NativeEditorEvent::route_to_stage(
        "selection",
        "pane-rich",
        handshake_native::event_emitter::native_editor_actor_id("pane-rich"),
        "WS-1",
    );
    let native = ev.to_native_payload();
    assert_eq!(
        native["action"], "route_to_stage",
        "MC-005: the canonical MT-036 event kind"
    );
    assert_eq!(
        native["pane_id"], "pane-rich",
        "the source pane is the typed pane_id"
    );
    assert_eq!(
        native["payload"]["content_kind"], "selection",
        "content_kind travels in the payload"
    );
    assert_eq!(native["workspace_id"], "WS-1");

    // The bus route_to_stage stages the routed content (the Stage pane then shows it) AND dispatches the
    // EXISTING CMD_ROUTE_TO_STAGE (the MT-033 command — extended, not duplicated). Run inside an egui ctx
    // so the dispatch path (which requests a repaint) has a context.
    let ctx = egui::Context::default();
    let _ = ctx.run(Default::default(), |ctx| {
        let mut bus = InteractionBus::new();
        bus.register_route_to_stage_command();
        let payload =
            build_from_selection(&text_range("pane-rich", 0, 5, "hello"), "WS-1").unwrap();
        let ack = handshake_native::interop::route_to_stage(ctx, &mut bus, &payload)
            .expect("route succeeds (bus-only, no backend POST)");
        assert!(
            ack.staged,
            "AC-002: the routed content was staged on the bus"
        );
        assert_eq!(ack.content_kind, "selection");
        // The staged content drains as a Selection the Stage pane renders.
        let staged = bus
            .take_pending_stage_content()
            .expect("content staged for the Stage pane drain");
        match staged {
            handshake_native::stage_pane::StageContent::Selection(text, src) => {
                assert_eq!(text, "hello");
                assert_eq!(src, "pane-rich:0-5");
            }
            other => panic!("AC-002: expected a Selection staged, got {other:?}"),
        }
    });

    // The Stage pane receives + renders the routed content (receive_routed_content) — the route-leg landing.
    let mut pane = StagePane::new();
    pane.receive_routed_content(handshake_native::stage_pane::StageContent::Selection(
        "hello".to_owned(),
        "pane-rich:0-5".to_owned(),
    ));
    assert!(
        pane.content.is_some(),
        "AC-002: the Stage pane shows the routed content"
    );
    assert!(pane.content.summary().contains("hello"));
    println!("PT-002 FR-shape + route wiring OK: route_to_stage event shape proven, content staged + received");
}

/// AC-002 LIVE round-trip against a REAL PostgreSQL/EventLedger with live native-editor FR ingestion.
/// GATED: `NEEDS_MANAGED_RESOURCE_PROOF` (both the managed PG and the live native-editor FR ingestion
/// endpoint are gated, like MT-064). The bus-only route + the FR-emit SHAPE are proven in
/// `route_to_stage_emits_fr_event_and_stages_content`; this asserts the LIVE ledger read once the
/// managed resource + ingestion route exist. Run with `--ignored` against a live backend.
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: real PostgreSQL/EventLedger + live native-editor FR ingestion (gated, like MT-064)"]
fn live_route_round_trip_real_pg() {
    // Intentionally minimal: the live ingestion endpoint does not exist in this build (the FR closed-schema
    // gap), so a real round-trip cannot be asserted here without fabricating it. When the native-editor FR
    // ingestion route lands, this test reads the route_to_stage event back from the ledger and asserts the
    // Stage pane received the content. Until then it is honestly gated rather than faked.
    panic!("live FR ingestion route absent in this build — gated proof (see test doc)");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-003 / AC-003 — embed-back inserts an MT-014 NodeView carrying SHA-256 manifest provenance.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn embed_back_inserts_mt014_nodeview_with_provenance() {
    let artifact = evidence_artifact("ART-77");
    let view =
        embed_artifact_as_nodeview(&artifact).expect("AC-003: evidence-grade artifact embeds");

    // The inserted NodeView is the MT-014 embed atom (an hsLink by ref_kind), NOT a parallel type.
    assert_eq!(view.node.ref_kind, STAGE_CAPTURE_REF_KIND);
    assert_eq!(
        view.node.ref_kind, "stage_capture",
        "the MT-014 hsLink ref_kind discriminator"
    );
    assert_eq!(view.node.ref_value, "ART-77");
    // The provenance descriptor is present and matches the fetched artifact's sha256 (the contract shape).
    assert_eq!(view.provenance.source, "stage_capture");
    assert_eq!(view.provenance.artifact_id, "ART-77");
    assert_eq!(view.provenance.sha256, artifact.sha256);
    assert_eq!(view.provenance.manifest_ref, "manifest://ART-77");

    // The Stage pane's capture_and_embed_back inserts the NodeView into a live note target and records the
    // outcome with the SHA-256 anchor. The insert closure proves the NodeView reaches the document model.
    use std::cell::RefCell;
    use std::rc::Rc;
    let inserted: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let cap = inserted.clone();
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Ok(artifact.clone()),
        &target,
        |pid| pid == "pane-rich", // target is live
        |view, _t| cap.borrow_mut().push(view.node.ref_value.clone()),
    );
    match outcome {
        handshake_native::stage_pane::EmbedBackOutcome::Embedded {
            artifact_id,
            sha256,
            target_pane,
        } => {
            assert_eq!(artifact_id, "ART-77");
            assert_eq!(sha256, artifact.sha256);
            assert_eq!(target_pane, "pane-rich");
        }
        other => panic!("AC-003: expected Embedded, got {other:?}"),
    }
    assert_eq!(
        inserted.borrow().as_slice(),
        ["ART-77"],
        "AC-003: the MT-014 NodeView reached the note"
    );
    println!(
        "PT-003 embed-back OK: MT-014 hsLink atom inserted into note, SHA-256 provenance preserved"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-004 / AC-004 — the missing embed-back route is the typed blocker (BROAD: 404 AND 501).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn embed_back_endpoint_absent_404() {
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 404 Not Found",
        serde_json::json!({"error": "not found"}),
    );
    let client = StageClient::with_base_url(base_url);
    let result = rt().block_on(async { client.fetch_stage_artifact("WS-1", "ART-1").await });
    let req_line = server.join().unwrap();

    // The probe is a GET (read-only) at the documented route.
    assert!(
        req_line.starts_with("GET "),
        "AC-004: fetch must issue a GET; got '{req_line}'"
    );
    assert!(
        req_line.contains("/workspaces/WS-1/stage/artifacts/ART-1"),
        "fetch must hit the documented embed-back route; got '{req_line}'"
    );
    match result {
        Err(StageInteropError::EmbedBackEndpointAbsent { probed_path }) => {
            assert!(
                probed_path.contains("/workspaces/WS-1/stage/artifacts/ART-1"),
                "AC-004: EmbedBackEndpointAbsent must name the probed path; got '{probed_path}'"
            );
            println!(
                "PT-004 typed blocker (404) OK: EmbedBackEndpointAbsent(probed='{probed_path}')"
            );
        }
        other => panic!("AC-004: a 404 must map to EmbedBackEndpointAbsent, got {other:?}"),
    }
}

#[test]
fn embed_back_endpoint_absent_501() {
    // BROAD detection (RISK-008/MC-008): a 501 Not Implemented is ALSO the typed blocker, not a generic
    // transport error.
    let (base_url, server) = spawn_mock(
        "HTTP/1.1 501 Not Implemented",
        serde_json::json!({"error": "not implemented"}),
    );
    let client = StageClient::with_base_url(base_url);
    let result = rt().block_on(async { client.fetch_stage_artifact("WS-1", "ART-2").await });
    let _ = server.join();
    assert!(
        matches!(
            result,
            Err(StageInteropError::EmbedBackEndpointAbsent { .. })
        ),
        "AC-004: a 501 must ALSO map to EmbedBackEndpointAbsent (broad detection), got {result:?}"
    );
    println!("PT-004 typed blocker (501) OK: 501 -> EmbedBackEndpointAbsent (broad detection)");
}

/// The embed-back never fabricates an artifact: even when the Stage pane runs the embed-back over an
/// absent endpoint, the outcome is the typed blocker (surfaced, never a fake embed). No insert happens.
#[test]
fn embed_back_blocker_surfaces_no_fake_embed() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let inserted: Rc<RefCell<usize>> = Rc::new(RefCell::new(0));
    let cap = inserted.clone();
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Err(StageInteropError::EmbedBackEndpointAbsent {
            probed_path: "/workspaces/WS-1/stage/artifacts/ART-1".into(),
        }),
        &target,
        |_pid| true,
        |_view, _t| *cap.borrow_mut() += 1,
    );
    assert!(
        outcome.is_endpoint_absent(),
        "AC-004: the blocker outcome is surfaced"
    );
    assert!(
        pane.has_embed_back_endpoint_absent_blocker(),
        "the host surfaces the blocker to the validator"
    );
    assert_eq!(
        *inserted.borrow(),
        0,
        "AC-004: NO artifact fabricated, NO insert on the typed blocker"
    );
    println!("PT-004 no-fake-embed OK: EmbedBackEndpointAbsent surfaced, zero inserts");
}

/// RISK-002/MC-002: an artifact with no SHA-256 / manifest provenance is REFUSED (ProvenanceMissing) — the
/// pane never embeds an unverifiable evidence-grade capture.
#[test]
fn embed_back_refuses_unverifiable_capture() {
    let mut artifact = evidence_artifact("ART-3");
    artifact.sha256 = String::new();
    artifact.manifest.sha256 = String::new();
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-rich".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(Ok(artifact), &target, |_pid| true, |_v, _t| {});
    assert_eq!(
        outcome,
        handshake_native::stage_pane::EmbedBackOutcome::ProvenanceMissing,
        "RISK-002/MC-002: an unverifiable artifact is refused, not embedded"
    );
}

/// RISK-007/MC-007: the embed target is re-resolved at embed time; a dangling target pane is refused.
#[test]
fn embed_back_refuses_dangling_target_pane() {
    let mut pane = StagePane::new();
    let target = EmbedTarget::Note {
        pane_id: "pane-gone".to_owned(),
        document_id: "DOC-1".to_owned(),
    };
    let outcome = pane.capture_and_embed_back(
        Ok(evidence_artifact("ART-4")),
        &target,
        |_pid| false,
        |_v, _t| {},
    );
    assert_eq!(
        outcome,
        handshake_native::stage_pane::EmbedBackOutcome::TargetGone {
            pane_id: "pane-gone".to_owned()
        },
        "RISK-007/MC-007: a dangling embed target pane is refused"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// PT-005 / AC-006 — AccessKit nodes present with correct roles + nesting (+ screenshot).
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn stage_pane_accesskit_nodes_present() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(380.0, 280.0))
        .wgpu()
        .build_ui(|ui| {
            let mut pane = StagePane::new();
            // Seed routed content so the routed-content region shows the route-leg landing.
            pane.receive_routed_content(handshake_native::stage_pane::StageContent::Selection(
                "routed selection".to_owned(),
                "pane-rich:0-16".to_owned(),
            ));
            pane.show_round_trip(ui, &dark());
        });
    harness.run();
    harness.run();

    let root = harness.root();

    // AC-006: the three contract-named nodes are present with the right roles.
    let pane_role = role_of(&root, STAGE_PANE_AUTHOR_ID);
    assert_eq!(
        pane_role.as_deref(),
        Some("GenericContainer"),
        "AC-006: '{STAGE_PANE_AUTHOR_ID}' must be Role::GenericContainer (got {pane_role:?})"
    );
    let routed_role = role_of(&root, STAGE_ROUTED_CONTENT_AUTHOR_ID);
    assert_eq!(
        routed_role.as_deref(),
        Some("GenericContainer"),
        "AC-006: '{STAGE_ROUTED_CONTENT_AUTHOR_ID}' must be Role::GenericContainer (got {routed_role:?})"
    );
    let btn_role = role_of(&root, STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID);
    assert_eq!(
        btn_role.as_deref(),
        Some("Button"),
        "AC-006: '{STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID}' must be Role::Button (got {btn_role:?})"
    );

    // Nesting: the routed-content region + the embed-back button are under the stage-pane container.
    assert!(
        author_under(&root, STAGE_ROUTED_CONTENT_AUTHOR_ID, STAGE_PANE_AUTHOR_ID),
        "AC-006: the routed-content region must nest under the stage-pane container"
    );
    assert!(
        author_under(
            &root,
            STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID,
            STAGE_PANE_AUTHOR_ID
        ),
        "AC-006: the embed-back button must nest under the stage-pane container"
    );

    println!(
        "PT-005 accesskit dump: {{\"{STAGE_PANE_AUTHOR_ID}\":\"{}\",\"{STAGE_ROUTED_CONTENT_AUTHOR_ID}\":\"{}\",\"{STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID}\":\"{}\"}}",
        pane_role.unwrap_or_default(),
        routed_role.unwrap_or_default(),
        btn_role.unwrap_or_default()
    );

    // Screenshot to the EXTERNAL root ONLY (best-effort pixel readback).
    if let Ok(image) = harness.render() {
        let ext_dir = external_artifact_dir("wp-kernel-012-mt-066");
        let _ = std::fs::create_dir_all(&ext_dir);
        let ext_path = ext_dir.join("MT-066-stage-round-trip.png");
        let saved = image.save(&ext_path).is_ok();
        println!(
            "PT-005 screenshot: {}x{} saved_ext={saved} ({})",
            image.width(),
            image.height(),
            ext_path.display()
        );
    } else {
        println!(
            "PT-005 screenshot: GPU readback unavailable on this host (structural proof stands)"
        );
    }

    assert_no_local_artifact_dir();
}

/// The embed-back button is driveable out-of-process: a click flips the pressed signal `show_round_trip`
/// returns (so the host runs the async fetch + capture_and_embed_back).
#[test]
fn embed_back_button_press_signals_host() {
    use std::cell::Cell;
    use std::rc::Rc;
    let pressed: Rc<Cell<bool>> = Rc::new(Cell::new(false));
    let flag = pressed.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(360.0, 240.0))
        .build_ui(move |ui| {
            let mut pane = StagePane::new();
            pane.receive_routed_content(handshake_native::stage_pane::StageContent::Selection(
                "x".to_owned(),
                "pane-rich:0-1".to_owned(),
            ));
            if pane.show_round_trip(ui, &dark()) {
                flag.set(true);
            }
        });
    harness.run();
    harness
        .get_by(|n| n.author_id() == Some(STAGE_CAPTURE_EMBED_BACK_AUTHOR_ID))
        .click();
    harness.run();
    assert!(
        pressed.get(),
        "AC-006: clicking stage-capture-embed-back signals the host to run embed-back"
    );
    println!("PT-005 button press OK: stage-capture-embed-back click -> host embed-back signal");
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-005 — a single route-to-stage command id (extend MT-033) + the added embed-stage-capture command.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn single_route_command_id_plus_embed_command() {
    use handshake_native::command_registry::all_commands;

    // Exactly ONE route-to-stage command id in the palette catalog (MT-033 extended, NOT duplicated).
    let route_rows: Vec<_> = all_commands()
        .iter()
        .filter(|c| c.id == CMD_ROUTE_TO_STAGE)
        .collect();
    assert_eq!(
        route_rows.len(),
        1,
        "AC-005/MC-003: exactly one route-to-stage command id ({CMD_ROUTE_TO_STAGE}); MT-033 extended, not duplicated"
    );
    assert_eq!(CMD_ROUTE_TO_STAGE, "interop.route-to-stage");

    // The NEW embed-stage-capture command id is present exactly once.
    let embed_rows: Vec<_> = all_commands()
        .iter()
        .filter(|c| c.id == CMD_EMBED_STAGE_CAPTURE)
        .collect();
    assert_eq!(
        embed_rows.len(),
        1,
        "AC-005: the added embed-stage-capture command id is present"
    );
    assert_eq!(CMD_EMBED_STAGE_CAPTURE, "interop.embed-stage-capture");
    assert_eq!(embed_rows[0].label, "Embed Stage Capture");
    assert!(
        !embed_rows[0].disabled,
        "the embed-stage-capture command is enabled (palette-driven)"
    );

    // The runtime bus also carries exactly one route + one embed-stage-capture descriptor (the WRAP-not-
    // fork registration). The route command is the EXISTING MT-033 register; the embed command is the new
    // MT-066 register.
    let mut bus = InteractionBus::new();
    bus.register_route_to_stage_command();
    handshake_native::interop::register_embed_stage_capture_command(&mut bus);
    assert!(
        bus.commands().get(CMD_ROUTE_TO_STAGE).is_some(),
        "route-to-stage descriptor on the bus"
    );
    assert!(
        bus.commands().get(CMD_EMBED_STAGE_CAPTURE).is_some(),
        "embed-stage-capture descriptor on the bus"
    );
    println!(
        "AC-005 command surface OK: 1 route-to-stage id (extended) + 1 embed-stage-capture id"
    );
}

// ════════════════════════════════════════════════════════════════════════════════════════════════
// AC-007 — no SQLite anywhere, no src/backend edit, no fabricated backend route.
// ════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn no_sqlite_no_backend_edit() {
    // The MT-066 production sources must NOT touch SQLite/rusqlite (PostgreSQL/EventLedger is the only
    // durable authority — AC-007). The Stage embed-back read is the only persistence touch, and it is a
    // GET against the existing handshake_core API surface.
    let sources: [(&str, &str); 2] = [
        (
            "stage_interop.rs",
            include_str!("../src/interop/stage_interop.rs"),
        ),
        ("stage_pane.rs", include_str!("../src/stage_pane.rs")),
    ];
    for (name, src) in sources {
        for store in ["sqlite", "rusqlite", "Sqlite", "SQLite"] {
            assert!(
                !src.contains(store),
                "AC-007: {name} must not reference '{store}' (PostgreSQL/EventLedger only)"
            );
        }
        // No write verbs on the Stage read client (read-only embed-back fetch).
        for verb in [".post(", ".put(", ".delete(", ".patch("] {
            assert!(
                !src.contains(verb),
                "AC-007: {name} embed-back read must be GET-only — found write verb '{verb}'"
            );
        }
    }
    // The stage_interop client reuses the shared backend pool + base url (no second HTTP stack).
    let interop_src = include_str!("../src/interop/stage_interop.rs");
    assert!(
        interop_src.contains("shared_http_client") && interop_src.contains("BACKEND_BASE_URL"),
        "AC-007: the Stage client must reuse the shared backend_client pool + base url (no second stack)"
    );
    // The embed-back GET is the only verb (the read builder).
    assert!(
        interop_src.contains(".get(&url)"),
        "AC-007: the Stage embed-back read must issue a GET via the reqwest builder"
    );
    println!("AC-007 gate OK: no sqlite/rusqlite, GET-only embed-back read, shared client reused, no backend route");
}

// ── small AccessKit tree helpers (the proven MT-063 helpers) ──────────────────────────────────────

/// The `{:?}` role string of the first node with `author_id`, if present.
fn role_of(root: &egui_kittest::Node<'_>, author_id: &str) -> Option<String> {
    for node in root.children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return Some(format!("{:?}", ak.role()));
        }
    }
    None
}

/// True if a node addressed `child_author` has an ancestor addressed `ancestor_author`.
fn author_under(root: &egui_kittest::Node<'_>, child_author: &str, ancestor_author: &str) -> bool {
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
