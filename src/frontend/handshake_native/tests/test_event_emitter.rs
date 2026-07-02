//! WP-KERNEL-012 MT-036 (E5 — one event ledger across surfaces) proof suite.
//!
//! Maps each MT-036 acceptance criterion to a real proof:
//!   - AC-4 (unit, no panic): a FAILED emit (backend unreachable / no runtime) lands in the cap-20
//!     in-memory error ring and does NOT crash the frame — `failed_emit_lands_in_error_ring`.
//!   - AC-5 (compile + structural): `surface_extension_seam.rs` compiles and its `EditorSurface` trait is
//!     OBJECT-SAFE (a `Box<dyn EditorSurface>` constructs) — `surface_extension_seam_is_object_safe`.
//!   - AC-6 (unit): an `EditorSurfaceRegistry` with a registered mock surface receives
//!     `on_selection_changed` AND `on_event_emitted` — `registry_dispatches_to_mock_surface`.
//!   - AC-7 (kittest): the `FlightRecorderPane` renders a `fr-event-*` ListItem under the
//!     `flight-recorder-pane` Region when an event exists — `flight_recorder_pane_lists_event`.
//!   - RISK-1 / MC-1 (unit): `build_post_body` carries every required `RuntimeChatEventV0_1` field with
//!     the exact snake_case key the backend's `deny_unknown_fields` handler demands —
//!     `post_body_matches_verified_runtime_chat_schema`.
//!   - AC-1/2/3 + PT-3/4/6 (REAL-PG round-trip): a TYPED BACKEND BLOCKER. The verified backend has NO
//!     ingestion endpoint that records a native-editor event with a custom actor/action (the
//!     `runtime_chat_event` endpoint is `deny_unknown_fields` + a closed 3-value `type` enum +
//!     hardcoded `actor_id="runtime_chat"`, and there is no native-editor `editor_edit`/`system` POST
//!     route — only the server-side Atelier-apply emit). The `--features integration` test documents the
//!     blocker honestly and is `#[ignore]` so CI never reports a fake pass — `native_editor_round_trip`.
//!   - AC-8: `cargo test -p handshake-native event_emitter` passes (this file + the lib unit tests).
//!
//! ## Artifact hygiene (CX-212E, HARD)
//!
//! The screenshot proof writes ONLY to the EXTERNAL artifact root via [`external_artifact_dir`];
//! [`assert_no_local_artifact_dir`] fails the run if a repo-local `test_output/` or `tests/screenshots/`
//! dir exists. NO artifact is ever written under `src/`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::event_emitter::{
    native_editor_actor_id, EmitError, ErrorRing, EventLedgerTransport, NativeEditorEvent,
    NativeEditorEventEmitter, RuntimeChatLedgerTransport, UndoScope, EMIT_PERMITS,
    FR_RUNTIME_CHAT_SCHEMA_VERSION,
};
use handshake_native::flight_recorder_pane::{
    fr_event_row_author_id, FlightRecorderPane, FlightRecorderQuery, FlightRecorderRow,
    FLIGHT_RECORDER_PANE_AUTHOR_ID,
};
use handshake_native::interop::interaction_bus::SharedSelection;
use handshake_native::surface_extension_seam::{
    EditorSurface, EditorSurfaceRegistry, UndoResult as SeamUndoResult,
};
use handshake_native::theme::HsTheme;

// ── Artifact hygiene (CX-212E, disk-agnostic) ────────────────────────────────────────────────────────

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree. `#[allow(dead_code)]` so the no-feature build does not warn.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (CX-212E hygiene). Checks BOTH
/// `test_output/` and `tests/screenshots/`.
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

/// Collect every author_id present in the live AccessKit tree.
fn author_ids<S>(harness: &Harness<'_, S>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

// ── Test doubles ──────────────────────────────────────────────────────────────────────────────────

/// An in-memory transport that records bodies + can force a failure (never touches the network).
struct MockTransport {
    posted: Arc<Mutex<Vec<serde_json::Value>>>,
    fail: bool,
}
impl MockTransport {
    fn new(fail: bool) -> Self {
        Self {
            posted: Arc::new(Mutex::new(Vec::new())),
            fail,
        }
    }
}
impl EventLedgerTransport for MockTransport {
    fn build_post_body(&self, event: &NativeEditorEvent) -> serde_json::Value {
        RuntimeChatLedgerTransport::with_session_id("http://test", uuid_session())
            .build_post_body(event)
    }
    fn post(
        &self,
        event: NativeEditorEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>> {
        let posted = Arc::clone(&self.posted);
        let fail = self.fail;
        let body = self.build_post_body(&event);
        Box::pin(async move {
            if fail {
                Err(EmitError::Transport("forced".to_owned()))
            } else {
                posted.lock().unwrap().push(body);
                Ok(())
            }
        })
    }
}

/// A valid non-nil UUID string for the transport session id (the backend requires a UUID session_id).
fn uuid_session() -> String {
    "11111111-1111-4111-8111-111111111111".to_owned()
}

/// A mock future surface (proves the seam trait is object-safe + the registry dispatches callbacks).
struct MockSurface {
    selection_changes: Arc<Mutex<usize>>,
    events_observed: Arc<Mutex<Vec<String>>>,
}
impl EditorSurface for MockSurface {
    fn surface_id(&self) -> &'static str {
        "mock_spreadsheet"
    }
    fn on_selection_changed(&self, _selection: &SharedSelection) {
        *self.selection_changes.lock().unwrap() += 1;
    }
    fn on_event_emitted(&self, event: &NativeEditorEvent, _emitter: &NativeEditorEventEmitter) {
        self.events_observed
            .lock()
            .unwrap()
            .push(event.action.as_str().to_owned());
    }
    fn undo_local(&self) -> Option<SeamUndoResult> {
        None
    }
    fn redo_local(&self) -> Option<SeamUndoResult> {
        None
    }
}

/// A query that returns injected rows (the headless FlightRecorderPane path — no live backend).
struct InjectedRows(Vec<FlightRecorderRow>);
impl FlightRecorderQuery for InjectedRows {
    fn rows(&self) -> Result<Vec<FlightRecorderRow>, String> {
        Ok(self.0.clone())
    }
}

// ── RISK-1 / MC-1: the wire body matches the VERIFIED RuntimeChatEventV0_1 schema ────────────────────

#[test]
fn post_body_matches_verified_runtime_chat_schema() {
    let transport = RuntimeChatLedgerTransport::with_session_id("http://test", uuid_session());
    let ev = NativeEditorEvent::document_saved(
        "DOC-9",
        "a".repeat(64),
        "pane-rich",
        native_editor_actor_id("pane-rich"),
        "WS-7",
    );
    let body = transport.build_post_body(&ev);
    let obj = body.as_object().expect("body is a JSON object");

    assert_eq!(obj["schema_version"], FR_RUNTIME_CHAT_SCHEMA_VERSION);
    assert!(uuid::Uuid::parse_str(obj["event_id"].as_str().unwrap()).is_ok());
    assert!(chrono::DateTime::parse_from_rfc3339(obj["ts_utc"].as_str().unwrap()).is_ok());
    let sid = uuid::Uuid::parse_str(obj["session_id"].as_str().unwrap()).unwrap();
    assert_ne!(
        sid,
        uuid::Uuid::nil(),
        "session_id must be a NON-NIL UUID (backend 400s otherwise)"
    );
    assert_eq!(
        obj["type"], "runtime_chat_message_appended",
        "type is a closed-enum value"
    );
    assert_eq!(obj["wsid"], "WS-7");
    assert_eq!(obj["body_sha256"], "a".repeat(64));

    // deny_unknown_fields: ONLY allowed snake_case keys may appear.
    let allowed: std::collections::HashSet<&str> = [
        "schema_version",
        "event_id",
        "ts_utc",
        "session_id",
        "job_id",
        "work_packet_id",
        "spec_id",
        "wsid",
        "type",
        "message_id",
        "role",
        "model_role",
        "body_sha256",
        "ans001_sha256",
        "ans001_compliant",
        "violation_clauses",
    ]
    .into_iter()
    .collect();
    for k in obj.keys() {
        assert!(
            allowed.contains(k.as_str()),
            "key '{k}' would trip the backend deny_unknown_fields"
        );
    }
    println!("RISK-1/MC-1: build_post_body carries every required RuntimeChatEventV0_1 field, snake_case");
}

// ── AC-4: a failed emit lands in the error ring, no panic ─────────────────────────────────────────────

#[test]
fn failed_emit_lands_in_error_ring() {
    // No runtime (headless): emit cannot dispatch -> recorded as NoRuntime, frame survives.
    let emitter = NativeEditorEventEmitter::new("WS-1", Arc::new(MockTransport::new(false)), None);
    let res = emitter.emit_document_saved("DOC-1", "h".repeat(64), "pane-rich");
    assert_eq!(res, Err(EmitError::NoRuntime("document_saved".to_owned())));
    assert_eq!(emitter.error_ring().len(), 1);
    assert_eq!(
        emitter.available_permits(),
        EMIT_PERMITS,
        "permit released, not leaked"
    );
    println!(
        "AC-4: a failed emit is logged to the cap-20 error ring with no panic / no frame block"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn failed_post_with_runtime_lands_in_error_ring() {
    let emitter = NativeEditorEventEmitter::new(
        "WS-1",
        Arc::new(MockTransport::new(true)), // forced transport failure.
        Some(tokio::runtime::Handle::current()),
    );
    emitter
        .emit_undo_fired(UndoScope::Local, "pane-rich")
        .expect("dispatched");
    for _ in 0..100 {
        if !emitter.error_ring().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        emitter.error_ring().len(),
        1,
        "a forced transport failure is recorded, not panicked"
    );
    assert!(matches!(
        emitter.error_ring().entries()[0].error,
        EmitError::Transport(_)
    ));
}

// ── AC-5: the extension seam compiles + the EditorSurface trait is object-safe ────────────────────────

#[test]
fn surface_extension_seam_is_object_safe() {
    // If EditorSurface were not object-safe this would not COMPILE — AC-5's compile proof, made explicit.
    let _boxed: Box<dyn EditorSurface> = Box::new(MockSurface {
        selection_changes: Arc::new(Mutex::new(0)),
        events_observed: Arc::new(Mutex::new(Vec::new())),
    });
    assert_eq!(_boxed.surface_id(), "mock_spreadsheet");
    println!(
        "AC-5: surface_extension_seam compiles; EditorSurface is object-safe (Box<dyn> constructs)"
    );
}

// ── AC-6: the registry dispatches selection + event callbacks to a registered mock surface ────────────

#[test]
fn registry_dispatches_to_mock_surface() {
    let selection_changes = Arc::new(Mutex::new(0usize));
    let events_observed = Arc::new(Mutex::new(Vec::new()));
    let mut reg = EditorSurfaceRegistry::new();
    assert!(reg.is_empty(), "registry starts empty (production state)");
    reg.register_surface(Box::new(MockSurface {
        selection_changes: Arc::clone(&selection_changes),
        events_observed: Arc::clone(&events_observed),
    }));
    assert_eq!(reg.len(), 1);

    reg.dispatch_selection_changed(&SharedSelection::None);
    assert_eq!(
        *selection_changes.lock().unwrap(),
        1,
        "on_selection_changed fired"
    );

    let emitter = NativeEditorEventEmitter::new(
        "WS-1",
        Arc::new(RuntimeChatLedgerTransport::new("http://test")),
        None,
    );
    let event =
        NativeEditorEvent::document_saved("DOC-1", "h".repeat(64), "pane-rich", "act", "WS-1");
    reg.dispatch_event_emitted(&event, &emitter);
    assert_eq!(
        events_observed.lock().unwrap().as_slice(),
        &["document_saved".to_owned()],
        "on_event_emitted fired with the document_saved action"
    );
    println!("AC-6: a registered mock surface received on_selection_changed AND on_event_emitted");
}

// ── AC-7: the FlightRecorderPane lists a fr-event-* ListItem under the flight-recorder-pane Region ────

#[test]
fn flight_recorder_pane_lists_event() {
    let row = FlightRecorderRow {
        event_id: "FR-EVT-001".to_owned(),
        action: "document_saved".to_owned(),
        actor_id: native_editor_actor_id("pane-rich"),
        ts_utc: "2026-06-23T00:00:00Z".to_owned(),
    };
    let query = Arc::new(InjectedRows(vec![row.clone()]));
    let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
    pane.load_now(); // resolve to Loaded(rows) — no perpetual spinner.

    let pane = Arc::new(pane);
    let pane_ui = Arc::clone(&pane);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 320.0))
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            pane_ui.show(ui, &pal);
        });
    harness.run();

    let ids = author_ids(&harness);
    assert!(
        ids.contains(FLIGHT_RECORDER_PANE_AUTHOR_ID),
        "AC-7: live tree must contain the '{FLIGHT_RECORDER_PANE_AUTHOR_ID}' Region; got {ids:?}"
    );
    let expected_row_id = fr_event_row_author_id("FR-EVT-001");
    assert!(
        ids.contains(&expected_row_id),
        "AC-7: live tree must contain a '{expected_row_id}' ListItem after a document_saved exists; got {ids:?}"
    );

    // Verify the roles are field-correct (Region root + ListItem rows).
    let mut region_role = String::new();
    let mut row_role = String::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        match ak.author_id() {
            Some(a) if a == FLIGHT_RECORDER_PANE_AUTHOR_ID => {
                region_role = format!("{:?}", ak.role())
            }
            Some(a) if a == expected_row_id => row_role = format!("{:?}", ak.role()),
            _ => {}
        }
    }
    assert_eq!(
        region_role, "Region",
        "flight-recorder-pane must be Role::Region"
    );
    assert_eq!(
        row_role, "ListItem",
        "fr-event-* row must be Role::ListItem"
    );
    println!("AC-7: FlightRecorderPane lists '{expected_row_id}' (ListItem) under '{FLIGHT_RECORDER_PANE_AUTHOR_ID}' (Region)");
}

// ── HBR-VIS screenshot (best-effort GPU; structural proofs stand without a GPU) ───────────────────────

#[cfg(feature = "wgpu_screenshots")]
#[test]
fn flight_recorder_pane_screenshot() {
    let row = FlightRecorderRow {
        event_id: "FR-EVT-SHOT".to_owned(),
        action: "document_saved".to_owned(),
        actor_id: native_editor_actor_id("pane-rich"),
        ts_utc: "2026-06-23T00:00:00Z".to_owned(),
    };
    let query = Arc::new(InjectedRows(vec![row]));
    let mut pane = FlightRecorderPane::new(query, ErrorRing::new());
    pane.load_now();
    let pane = Arc::new(pane);
    let pane_ui = Arc::clone(&pane);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 320.0))
        .wgpu()
        .build_ui(move |ui| {
            let pal = HsTheme::Dark.palette();
            pane_ui.show(ui, &pal);
        });
    harness.run();
    match harness.render() {
        Ok(image) => {
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-036");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png = ext_dir.join("MT-036-flight-recorder-pane.png");
            let saved = image.save(&png).is_ok();
            println!(
                "HBR-VIS: {}x{} screenshot saved={saved} ({})",
                image.width(),
                image.height(),
                png.display()
            );
        }
        Err(e) => {
            println!("BLOCKER(non-fatal): FR pane screenshot render unavailable (no wgpu adapter): {e}. The AccessKit proof passed; the PNG is a GPU-host item.");
        }
    }
    assert_no_local_artifact_dir();
}

// ── LIVE melt-together emit path: the InteractionBus emit_event + route_to_stage call sites ───────────

#[tokio::test(flavor = "multi_thread")]
async fn bus_emit_event_dispatches_to_installed_emitter() {
    // The melt-together path the rich-pane save/undo live call sites use: bus.emit_event() routes to the
    // installed emitter (and fans out to the empty future-surface registry — a production no-op).
    use handshake_native::interop::interaction_bus::InteractionBus;
    let mock = Arc::new(MockTransport::new(false));
    let emitter = NativeEditorEventEmitter::new(
        "WS-LIVE",
        mock.clone(),
        Some(tokio::runtime::Handle::current()),
    );
    let mut bus = InteractionBus::new();
    // Before installing the emitter, emit_event is an HONEST no-op (the unmounted-pane defer policy).
    assert!(
        !bus.emit_event(NativeEditorEvent::undo_fired(
            UndoScope::Local,
            "pane-rich",
            "a",
            "WS-LIVE"
        )),
        "emit_event must be a no-op (false) before the emitter is installed"
    );
    bus.set_event_emitter(emitter);
    // After install, an undo_fired emit dispatches through the bus to the transport.
    assert!(
        bus.emit_event(NativeEditorEvent::undo_fired(
            UndoScope::Local,
            "pane-rich",
            "a",
            "WS-LIVE"
        )),
        "emit_event must dispatch (true) once the emitter is installed"
    );
    for _ in 0..100 {
        if !mock.posted.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let posted = mock.posted.lock().unwrap();
    assert_eq!(
        posted.len(),
        1,
        "the installed emitter posted the undo_fired event"
    );
    assert_eq!(posted[0]["message_id"], "native_editor:undo_fired");
}

#[tokio::test(flavor = "multi_thread")]
async fn bus_route_to_stage_emits_route_event() {
    // The LIVE MT-033 route-to-stage call site (a public bus method) emits a route_to_stage event.
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::stage_pane::StageContent;
    let mock = Arc::new(MockTransport::new(false));
    let emitter = NativeEditorEventEmitter::new(
        "WS-LIVE",
        mock.clone(),
        Some(tokio::runtime::Handle::current()),
    );
    let ctx = egui::Context::default();
    let mut bus = InteractionBus::new();
    bus.set_event_emitter(emitter);
    bus.register_route_to_stage_command();
    let _ = bus.route_to_stage(
        &ctx,
        StageContent::Selection("hi".to_owned(), "DOC-1".to_owned()),
    );
    for _ in 0..100 {
        if !mock.posted.lock().unwrap().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let posted = mock.posted.lock().unwrap();
    assert_eq!(
        posted.len(),
        1,
        "route_to_stage emitted exactly one event at the live call site"
    );
    assert_eq!(posted[0]["message_id"], "native_editor:route_to_stage");
}

// ── AC-8 hygiene guard (always runs) ─────────────────────────────────────────────────────────────────

#[test]
fn no_repo_local_artifact_dir() {
    assert_no_local_artifact_dir();
}

// ── AC-1/2/3 + PT-3/4/6: REAL-PG native-editor round-trip (TYPED BACKEND BLOCKER) ─────────────────────

/// The native-editor → Flight Recorder round-trip (save a rich doc, query the ledger, assert a
/// `document_saved` native event present, filtered by the native actor).
///
/// THIS IS A TYPED BACKEND BLOCKER, not a fake. Verification of the FROZEN backend
/// (`src/backend/handshake_core/src/api/flight_recorder.rs` + `api/workspaces.rs`) shows there is NO
/// HTTP ingestion endpoint that records a native-editor event with a custom `actor_id`/action:
///   - `POST /flight_recorder/runtime_chat_event` is `deny_unknown_fields`, has a CLOSED 3-value `type`
///     enum (no `system`), forces `session_id` to be a non-nil UUID, and HARDCODES `actor_id="runtime_chat"`;
///   - the only `editor_edit` FlightEvents the ledger holds are emitted SERVER-SIDE from the Atelier-apply
///     endpoint (hardwired `editor_surface="monaco"`, `actor=Human`);
///   - `GET /flight_recorder` has NO `actor_id` filter.
/// Therefore a native-editor `document_saved` (action + native actor + pane_id) CANNOT be POSTed and then
/// queried back by `actor_id='native_editor_human'` without a NEW backend ingestion endpoint
/// (`POST /flight_recorder/native_editor_event` recording a `FlightRecorderEventType::EditorEdit` with a
/// native actor/surface, queryable by `actor`/`surface`). Backend edits are out of scope
/// (`src/backend/** = reuse-via-API-only`).
///
/// `#[ignore]` + `--features integration` so CI NEVER reports a fake pass. When the backend ingestion
/// endpoint lands, swap `RuntimeChatLedgerTransport` for the native-editor transport and replace the
/// `panic!` below with the real assert (the emitter + body are already correct + live-wired).
#[cfg(feature = "integration")]
#[test]
#[ignore = "TYPED BACKEND BLOCKER: no native-editor Flight Recorder ingestion endpoint (see fn doc + event_emitter.rs)"]
fn native_editor_round_trip() {
    panic!(
        "MT-036 round-trip is BLOCKED on a missing backend ingestion endpoint. The verified backend has \
         no HTTP route that records a native-editor FlightEvent with a custom actor_id/action \
         (runtime_chat_event is deny_unknown_fields + closed type-enum + hardcoded actor_id; editor_edit \
         is server-side-only from Atelier-apply; GET has no actor_id filter). This requires a NEW backend \
         endpoint (POST /flight_recorder/native_editor_event) — out of scope (src/backend reuse-only), \
         routed as a typed blocker. The emitter, body shape, bounded spawn, error ring, and the LIVE \
         save/undo/route emit call sites are all REAL and proven by the non-ignored tests."
    );
}
