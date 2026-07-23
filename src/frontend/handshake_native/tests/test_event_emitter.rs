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
//!   - RISK-1 / MC-1 (unit): `build_post_body` carries every required native-editor envelope field with
//!     the exact snake_case key the backend's `deny_unknown_fields` handler demands —
//!     `post_body_matches_verified_native_editor_schema`.
//!   - AC-1/2/3 + PT-3/4/6 (live round-trip): three native actions POST through the production transport
//!     and are read back from the real Flight Recorder route with exact actor/action/workspace ordering.
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
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::event_emitter::{
    native_editor_actor_id, EmitError, EmitErrorEntry, ErrorRing, EventLedgerTransport,
    NativeEditorEvent, NativeEditorEventEmitter, RuntimeChatLedgerTransport, UndoScope,
    EMIT_PERMITS, NATIVE_EDITOR_SCHEMA_VERSION, NATIVE_EDITOR_WORK_PACKET_ID,
};
use handshake_native::flight_recorder_pane::{
    fr_event_row_author_id, FlightRecorderPane, FlightRecorderQuery, FlightRecorderQueryRows,
    FlightRecorderRow, FLIGHT_RECORDER_ERROR_RING_AUTHOR_ID,
    FLIGHT_RECORDER_ERROR_ROW_AUTHOR_PREFIX, FLIGHT_RECORDER_LOAD_FAILURE_AUTHOR_ID,
    FLIGHT_RECORDER_PANE_AUTHOR_ID, FLIGHT_RECORDER_QUARANTINE_STATUS_AUTHOR_ID,
    FLIGHT_RECORDER_REFRESH_AUTHOR_ID,
};
use handshake_native::interop::interaction_bus::SharedSelection;
use handshake_native::quick_switcher::ShellNavigator;
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

struct SlowTransport;

impl EventLedgerTransport for SlowTransport {
    fn build_post_body(&self, event: &NativeEditorEvent) -> serde_json::Value {
        RuntimeChatLedgerTransport::with_session_id("http://test", uuid_session())
            .build_post_body(event)
    }

    fn post(
        &self,
        _event: NativeEditorEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>> {
        Box::pin(async move {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            Ok(())
        })
    }
}

/// Records the exact transport/session generation selected by the InteractionBus.
struct SessionMockTransport {
    session_id: String,
    posted: Arc<Mutex<Vec<serde_json::Value>>>,
}

impl SessionMockTransport {
    fn new(session_id: &str) -> Self {
        Self {
            session_id: session_id.to_owned(),
            posted: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl EventLedgerTransport for SessionMockTransport {
    fn build_post_body(&self, event: &NativeEditorEvent) -> serde_json::Value {
        RuntimeChatLedgerTransport::with_session_id("http://test", self.session_id.clone())
            .build_post_body(event)
    }

    fn post(
        &self,
        event: NativeEditorEvent,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), EmitError>> + Send>> {
        let posted = Arc::clone(&self.posted);
        let body = self.build_post_body(&event);
        Box::pin(async move {
            posted.lock().unwrap().push(body);
            Ok(())
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
    fn rows(&self) -> Result<FlightRecorderQueryRows, String> {
        Ok(FlightRecorderQueryRows {
            rows: self.0.clone(),
            quarantined: Vec::new(),
        })
    }
}

struct InjectedQuery(Result<FlightRecorderQueryRows, String>);
impl FlightRecorderQuery for InjectedQuery {
    fn rows(&self) -> Result<FlightRecorderQueryRows, String> {
        self.0.clone()
    }
}

// ── RISK-1 / MC-1: the wire body matches the verified native-editor schema ──────────────────────────

#[test]
fn post_body_matches_verified_native_editor_schema() {
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

    assert_eq!(obj["schema_version"], NATIVE_EDITOR_SCHEMA_VERSION);
    assert!(uuid::Uuid::parse_str(obj["event_id"].as_str().unwrap()).is_ok());
    assert!(chrono::DateTime::parse_from_rfc3339(obj["ts_utc"].as_str().unwrap()).is_ok());
    let sid = uuid::Uuid::parse_str(obj["session_id"].as_str().unwrap()).unwrap();
    assert_ne!(
        sid,
        uuid::Uuid::nil(),
        "session_id must be a NON-NIL UUID (backend 400s otherwise)"
    );
    assert_eq!(obj["kind"], "document_saved");
    assert_eq!(obj["actor_id"], "hsk:native_editor:pane-rich");
    assert_eq!(obj["actor_kind"], "human");
    assert_eq!(obj["pane_id"], "pane-rich");
    assert_eq!(obj["surface"], "pane-rich");
    assert_eq!(obj["workspace_id"], "WS-7");
    assert_eq!(obj["work_packet_id"], NATIVE_EDITOR_WORK_PACKET_ID);
    assert_eq!(obj["payload"]["content_hash"], "a".repeat(64));

    // deny_unknown_fields: ONLY allowed snake_case keys may appear.
    let allowed: std::collections::HashSet<&str> = [
        "schema_version",
        "event_id",
        "ts_utc",
        "session_id",
        "kind",
        "actor_id",
        "actor_kind",
        "pane_id",
        "surface",
        "workspace_id",
        "work_packet_id",
        "payload",
    ]
    .into_iter()
    .collect();
    for k in obj.keys() {
        assert!(
            allowed.contains(k.as_str()),
            "key '{k}' would trip the backend deny_unknown_fields"
        );
    }
    println!("RISK-1/MC-1: build_post_body carries every required native-editor field, snake_case");
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
        event_code: None,
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

#[test]
fn flight_recorder_retry_and_failure_surfaces_have_stable_argus_ids() {
    let ring = ErrorRing::new();
    ring.push(EmitErrorEntry {
        action: "document_saved".to_owned(),
        error: EmitError::Transport("backend unavailable".to_owned()),
    });
    let query = Arc::new(InjectedQuery(Ok(FlightRecorderQueryRows {
        rows: Vec::new(),
        quarantined: vec!["bad-fems-row: event_code mismatch".to_owned()],
    })));
    let mut pane = FlightRecorderPane::new(query, ring);
    pane.load_now();
    let pane = Arc::new(pane);
    let pane_ui = Arc::clone(&pane);
    let mut harness = Harness::builder().build_ui(move |ui| {
        pane_ui.show(ui, &HsTheme::Dark.palette());
    });
    harness.run();
    let ids = author_ids(&harness);
    for expected in [
        FLIGHT_RECORDER_REFRESH_AUTHOR_ID,
        FLIGHT_RECORDER_QUARANTINE_STATUS_AUTHOR_ID,
        FLIGHT_RECORDER_ERROR_RING_AUTHOR_ID,
        &format!("{FLIGHT_RECORDER_ERROR_ROW_AUTHOR_PREFIX}0"),
    ] {
        assert!(
            ids.contains(expected),
            "missing stable Flight Recorder id {expected}"
        );
    }

    let query = Arc::new(InjectedQuery(Err("backend unreachable".to_owned())));
    let mut failed = FlightRecorderPane::new(query, ErrorRing::new());
    failed.load_now();
    let failed = Arc::new(failed);
    let failed_ui = Arc::clone(&failed);
    let mut failure_harness = Harness::builder().build_ui(move |ui| {
        failed_ui.show(ui, &HsTheme::Dark.palette());
    });
    failure_harness.run();
    let failed_ids = author_ids(&failure_harness);
    assert!(failed_ids.contains(FLIGHT_RECORDER_REFRESH_AUTHOR_ID));
    assert!(failed_ids.contains(FLIGHT_RECORDER_LOAD_FAILURE_AUTHOR_ID));
}

#[test]
fn flight_recorder_parser_keeps_valid_rows_and_quarantines_malformed_neighbors() {
    let event_id = uuid::Uuid::new_v4().to_string();
    let body = serde_json::json!([
        {
            "event_id": event_id,
            "timestamp": "2026-07-16T00:00:00.123456Z",
            "event_type": "system",
            "actor_id": "native_editor_human",
            "wsids": ["ws-1"],
            "payload": {
                "event_family":"native_editor",
                "schema":"hsk.native_editor@0.1",
                "schema_version":"hsk.native_editor@0.1",
                "action":"document_saved",
                "kind":"document_saved",
                "pane_id":"pane-rich",
                "workspace_id":"ws-1",
                "actor_id":"native_editor_human",
                "ts_utc":"2026-07-16T00:00:00.123456789Z"
            }
        },
        {
            "event_id": "",
            "timestamp": "not-a-time",
            "event_type": "wrong",
            "actor_id": "",
            "payload": {"event_family":"native_editor"}
        },
        {
            "event_id": uuid::Uuid::new_v4().to_string(),
            "timestamp": "2026-07-16T00:00:00.123456Z",
            "event_type": "system",
            "actor_id": "native_editor_human",
            "wsids": ["ws-1"],
            "payload": {
                "event_family":"native_editor",
                "schema":"hsk.native_editor@0.1",
                "schema_version":"hsk.native_editor@0.1",
                "action":"document_saved",
                "kind":"document_saved",
                "pane_id":"pane-rich",
                "workspace_id":"ws-1",
                "actor_id":"native_editor_human",
                "ts_utc":"2026-07-16T00:00:00.123457Z"
            }
        }
    ]);
    let rows = handshake_native::editor_pane_factories::flight_recorder_rows_from_json(&body)
        .expect("one malformed row must not poison valid recorder history");
    assert_eq!(rows.rows.len(), 1);
    assert_eq!(rows.rows[0].action, "document_saved");
    assert_eq!(
        rows.quarantined.len(),
        2,
        "malformed rows and next-microsecond timestamp drift are quarantined"
    );
}

#[test]
fn flight_recorder_parser_projects_exact_fems_lifecycle_and_quarantines_schema_drift() {
    let workspace_id = "550e8400-e29b-41d4-a716-446655440001";
    let artifact = |suffix: &str| {
        serde_json::json!({
            "artifact_id": suffix,
            "path": format!("/workspaces/{workspace_id}/memory/artifacts/{suffix}")
        })
    };
    let envelope = |event_type: &str, payload: serde_json::Value| {
        serde_json::json!({
            "event_id": uuid::Uuid::new_v4().to_string(),
            "timestamp": "2026-07-22T12:00:00Z",
            "event_type": event_type,
            "actor_id": "hsk:backend:memory",
            "wsids": [workspace_id],
            "payload": payload
        })
    };
    let entity_refs = serde_json::json!([{
        "artefact_type": "workspace",
        "artefact_id": workspace_id,
        "selector": "self"
    }]);
    let extra_field = serde_json::json!({
        "type": "memory_write_reviewed",
        "event_code": "FR-EVT-MEM-002",
        "proposal_id": "proposal-extra",
        "decision": "approved",
        "reviewer_kind": "user",
        "unexpected": true
    });

    let body = serde_json::Value::Array(vec![
        envelope(
            "memory_write_proposed",
            serde_json::json!({
                "type": "memory_write_proposed",
                "event_code": "FR-EVT-MEM-001",
                "proposal_id": "proposal-1",
                "proposal_hash": "a".repeat(64),
                "artifact_ref": artifact("550e8400-e29b-41d4-a716-44665544000a"),
                "scope_refs": entity_refs.clone(),
                "op_count": 2,
                "requires_review_count": 1
            }),
        ),
        envelope(
            "memory_write_reviewed",
            serde_json::json!({
                "type": "memory_write_reviewed",
                "event_code": "FR-EVT-MEM-002",
                "proposal_id": "proposal-1",
                "decision": "approved",
                "reviewer_kind": "user",
                "commit_report_ref": artifact("550e8400-e29b-41d4-a716-44665544000f")
            }),
        ),
        envelope(
            "memory_write_committed",
            serde_json::json!({
                "type": "memory_write_committed",
                "event_code": "FR-EVT-MEM-003",
                "commit_id": "commit-1",
                "proposal_id": "proposal-1",
                "commit_report_hash": "b".repeat(64),
                "artifact_ref": artifact("550e8400-e29b-41d4-a716-44665544000b"),
                "changed_memory_ids_hash": "c".repeat(64)
            }),
        ),
        envelope(
            "memory_pack_built",
            serde_json::json!({
                "type": "memory_pack_built",
                "event_code": "FR-EVT-MEM-004",
                "pack_id": "pack-1",
                "memory_pack_hash": "d".repeat(64),
                "artifact_ref": artifact("550e8400-e29b-41d4-a716-44665544000c"),
                "memory_policy": "WORKSPACE_SCOPED",
                "scope_refs": entity_refs,
                "item_count": 1,
                "token_estimate": 32,
                "truncation_occurred": false
            }),
        ),
        envelope(
            "memory_item_status_changed",
            serde_json::json!({
                "type": "memory_item_status_changed",
                "event_code": "FR-EVT-MEM-005",
                "memory_id": "memory-1",
                "previous_status": "active",
                "new_status": "superseded",
                "reason": "supersede",
                "actor": "policy"
            }),
        ),
        envelope("memory_write_reviewed", extra_field),
        envelope(
            "memory_write_committed",
            serde_json::json!({
                "type": "memory_write_committed",
                "event_code": "FR-EVT-MEM-002",
                "commit_id": "commit-wrong-code",
                "proposal_id": "proposal-1",
                "commit_report_hash": "e".repeat(64),
                "artifact_ref": artifact("550e8400-e29b-41d4-a716-44665544000d"),
                "changed_memory_ids_hash": "f".repeat(64)
            }),
        ),
    ]);

    let result = handshake_native::editor_pane_factories::flight_recorder_rows_from_json(&body)
        .expect("FEMS parsing must isolate malformed neighbors");
    assert_eq!(result.rows.len(), 5);
    assert_eq!(
        result
            .rows
            .iter()
            .map(|row| row.event_code.as_deref().expect("FEMS event code"))
            .collect::<Vec<_>>(),
        vec![
            "FR-EVT-MEM-001",
            "FR-EVT-MEM-002",
            "FR-EVT-MEM-003",
            "FR-EVT-MEM-004",
            "FR-EVT-MEM-005"
        ]
    );
    assert_eq!(result.quarantined.len(), 2);
    assert!(result
        .quarantined
        .iter()
        .any(|reason| reason.contains("non-canonical fields")));
    assert!(result
        .quarantined
        .iter()
        .any(|reason| reason.contains("mismatched event_code")));
}

#[test]
fn frame_retry_preserves_causal_prefix_and_rejects_incoming_at_capacity() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("frame retry runtime");
    let transport = Arc::new(MockTransport::new(false));
    let emitter = NativeEditorEventEmitter::new(
        "ws-frame-order",
        transport.clone(),
        Some(runtime.handle().clone()),
    );
    let ring = emitter.error_ring().clone();
    let ctx = egui::Context::default();
    handshake_native::event_emitter::install_frame_error_ring(&ctx, ring.clone());
    let bus = handshake_native::interop::InteractionBus::get_or_init(&ctx);
    bus.lock().unwrap().set_event_emitter(emitter);

    let events = (0..=EMIT_PERMITS)
        .map(|index| {
            NativeEditorEvent::document_saved(
                format!("DOC-{index}"),
                "a".repeat(64),
                "pane-rich",
                "caller-draft",
                "ws-frame-order",
            )
        })
        .collect::<Vec<_>>();
    let accepted_ids = events[..EMIT_PERMITS]
        .iter()
        .map(|event| event.event_id.clone())
        .collect::<Vec<_>>();
    let rejected_id = events[EMIT_PERMITS].event_id.clone();

    let held = bus.lock().unwrap();
    for event in events {
        assert!(!handshake_native::event_emitter::dispatch_event_from_frame(
            &ctx, event
        ));
    }
    drop(held);
    assert!(handshake_native::event_emitter::flush_pending_frame_events(
        &ctx
    ));
    runtime.block_on(async {
        for _ in 0..100 {
            if transport.posted.lock().unwrap().len() == EMIT_PERMITS {
                break;
            }
            tokio::task::yield_now().await;
        }
    });

    let posted_ids = transport
        .posted
        .lock()
        .unwrap()
        .iter()
        .map(|body| body["event_id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    assert_eq!(posted_ids, accepted_ids, "retained FIFO never reorders");
    assert!(ring.entries().iter().any(|entry| matches!(
        &entry.error,
        EmitError::PendingOverflow { event_id, .. } if event_id == &rejected_id
    )));
}

#[test]
fn repeated_frame_backpressure_is_coalesced_per_event_id() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("backpressure coalescing runtime");
    let emitter = NativeEditorEventEmitter::new(
        "ws-backpressure",
        Arc::new(MockTransport::new(false)),
        Some(runtime.handle().clone()),
    );

    // The current-thread runtime is intentionally not entered yet, so its ordered worker cannot
    // drain and all queue permits are deterministically occupied.
    for index in 0..EMIT_PERMITS {
        assert!(emitter
            .emit(NativeEditorEvent::document_saved(
                format!("DOC-{index}"),
                "a".repeat(64),
                "pane-rich",
                "caller",
                "ws-backpressure",
            ))
            .is_ok());
    }
    let retried = NativeEditorEvent::document_saved(
        "DOC-RETRY",
        "b".repeat(64),
        "pane-rich",
        "caller",
        "ws-backpressure",
    );
    for _ in 0..8 {
        assert!(matches!(
            emitter.emit(retried.clone()),
            Err(EmitError::Backpressure(_))
        ));
    }
    let backpressure = emitter
        .error_ring()
        .entries()
        .into_iter()
        .filter(|entry| matches!(entry.error, EmitError::Backpressure(_)))
        .count();
    assert_eq!(
        backpressure, 1,
        "retries of one immutable event cannot evict distinct operator-visible errors"
    );
}

#[test]
fn persistence_receipt_distinguishes_persisted_transport_failure_and_timeout() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("persistence receipt runtime");
    let event = |workspace: &str| {
        NativeEditorEvent::document_saved(
            "DOC-receipt",
            "a".repeat(64),
            "pane-rich",
            "caller",
            workspace,
        )
    };

    let persisted = NativeEditorEventEmitter::new(
        "ws-persisted",
        Arc::new(MockTransport::new(false)),
        Some(runtime.handle().clone()),
    );
    assert!(runtime
        .block_on(
            persisted.emit_persisted(event("ws-persisted"), std::time::Duration::from_secs(1),)
        )
        .is_ok());

    let failed = NativeEditorEventEmitter::new(
        "ws-failed",
        Arc::new(MockTransport::new(true)),
        Some(runtime.handle().clone()),
    );
    assert!(matches!(
        runtime.block_on(
            failed.emit_persisted(event("ws-failed"), std::time::Duration::from_secs(1),)
        ),
        Err(EmitError::Transport(_))
    ));

    let timed_out = NativeEditorEventEmitter::new(
        "ws-timeout",
        Arc::new(SlowTransport),
        Some(runtime.handle().clone()),
    );
    assert!(matches!(
        runtime.block_on(
            timed_out.emit_persisted(event("ws-timeout"), std::time::Duration::from_millis(10),)
        ),
        Err(EmitError::PersistenceTimeout { timeout_ms: 10, .. })
    ));
}

#[test]
fn event_emitter_workspace_revisit_preserves_original_emitter_session_generation() {
    use handshake_native::interop::interaction_bus::InteractionBus;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("workspace generation runtime");
    let a1_transport = Arc::new(SessionMockTransport::new(
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1",
    ));
    let b_transport = Arc::new(SessionMockTransport::new(
        "bbbbbbbb-bbbb-4bbb-8bbb-bbbbbbbbbbb1",
    ));
    let a2_transport = Arc::new(SessionMockTransport::new(
        "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa2",
    ));
    let mut bus = InteractionBus::new();
    bus.set_event_emitter(NativeEditorEventEmitter::new(
        "workspace-A",
        a1_transport.clone(),
        Some(runtime.handle().clone()),
    ));

    // The completion captured its immutable A identity before the shell moved A -> B -> A.
    let delayed_a1_completion = NativeEditorEvent::document_saved(
        "DOC-A1",
        "a".repeat(64),
        "pane-rich",
        "caller",
        "workspace-A",
    );
    bus.set_event_emitter(NativeEditorEventEmitter::new(
        "workspace-B",
        b_transport,
        Some(runtime.handle().clone()),
    ));
    bus.set_event_emitter(NativeEditorEventEmitter::new(
        "workspace-A",
        a2_transport.clone(),
        Some(runtime.handle().clone()),
    ));
    assert!(bus.emit_event(delayed_a1_completion));

    runtime.block_on(async {
        for _ in 0..100 {
            if !a1_transport.posted.lock().unwrap().is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    });
    let a1 = a1_transport.posted.lock().unwrap();
    assert_eq!(a1.len(), 1, "the delayed A1 completion reached A1");
    assert_eq!(
        a1[0]["session_id"], "aaaaaaaa-aaaa-4aaa-8aaa-aaaaaaaaaaa1",
        "the delayed completion retained A1's trace/session generation"
    );
    assert!(
        a2_transport.posted.lock().unwrap().is_empty(),
        "revisiting A must not relabel A1 work onto a replacement A2 emitter"
    );
}

#[test]
fn event_emitter_workspace_generations_are_bounded_and_reclaimed() {
    use handshake_native::interop::interaction_bus::{
        InteractionBus, MAX_RETAINED_EVENT_EMITTER_WORKSPACES,
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("workspace reclamation runtime");
    let mut bus = InteractionBus::new();
    let mut transports = Vec::new();
    for index in 0..(MAX_RETAINED_EVENT_EMITTER_WORKSPACES + 3) {
        let transport = Arc::new(SessionMockTransport::new(&uuid_session()));
        transports.push(Arc::downgrade(&transport));
        bus.set_event_emitter(NativeEditorEventEmitter::new(
            format!("workspace-{index}"),
            transport,
            Some(runtime.handle().clone()),
        ));
    }
    assert_eq!(
        bus.retained_event_emitter_workspace_count(),
        MAX_RETAINED_EVENT_EMITTER_WORKSPACES,
        "the InteractionBus retains only the bounded recent workspace generations"
    );

    runtime.block_on(async {
        for _ in 0..100 {
            if transports[..3]
                .iter()
                .all(|transport| transport.upgrade().is_none())
            {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("evicted emitter workers retained their transport after sender reclamation");
    });
}

// ── HBR-VIS screenshot (best-effort GPU; structural proofs stand without a GPU) ───────────────────────

#[cfg(feature = "wgpu_screenshots")]
#[test]
fn flight_recorder_pane_screenshot() {
    let row = FlightRecorderRow {
        event_id: "FR-EVT-SHOT".to_owned(),
        action: "document_saved".to_owned(),
        event_code: None,
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
    assert_eq!(posted[0]["kind"], "undo_fired");
}

#[tokio::test(flavor = "multi_thread")]
async fn bus_route_to_stage_defers_receipt_until_stage_acknowledges() {
    // Bus admission alone is not success: the shell emits only after the mounted Stage pane applies and
    // acknowledges this exact prebuilt event.
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
    assert!(bus.route_to_stage(
        &ctx,
        StageContent::Selection("hi".to_owned(), "DOC-1".to_owned()),
    ));
    let pending = bus
        .pending_stage_route()
        .expect("route remains pending until mounted Stage applies it");
    assert_eq!(pending.content_kind, "selection");
    assert_eq!(
        pending.receipt.to_native_payload()["action"],
        "route_to_stage"
    );
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let posted = mock.posted.lock().unwrap();
    assert_eq!(
        posted.len(),
        0,
        "bus-only admission must not emit a false success receipt"
    );
}

// ── MT-036 unified-undo emit: EVERY undo path emits `undo_fired` exactly once with the right scope ────

/// A recording `EditorSurface` that captures each emitted event's `(action, scope)` SYNCHRONOUSLY via the
/// `InteractionBus::emit_event` fan-out. Deterministic (no async race): one callback == one ledger emit.
/// MT-036 uses it to prove EACH undo path emits `undo_fired` exactly once with the correct `UndoScope`
/// (the async ledger POST itself is proven separately by `bus_emit_event_dispatches_to_installed_emitter`,
/// but that path's closed post-body drops the scope — the native payload the surface sees carries it).
/// The `(action, scope)` capture log the recording surface appends to.
type UndoFiredLog = Arc<Mutex<Vec<(String, Option<String>)>>>;

struct ScopeRecordingSurface {
    seen: UndoFiredLog,
}
impl EditorSurface for ScopeRecordingSurface {
    fn surface_id(&self) -> &'static str {
        "mt036_scope_recorder"
    }
    fn on_selection_changed(
        &self,
        _s: &handshake_native::interop::interaction_bus::SharedSelection,
    ) {
    }
    fn on_event_emitted(&self, event: &NativeEditorEvent, _e: &NativeEditorEventEmitter) {
        let p = event.to_native_payload();
        let action = p["action"].as_str().unwrap_or_default().to_owned();
        let scope = p["payload"]["scope"].as_str().map(|s| s.to_owned());
        self.seen.lock().unwrap().push((action, scope));
    }
    fn undo_local(&self) -> Option<SeamUndoResult> {
        None
    }
    fn redo_local(&self) -> Option<SeamUndoResult> {
        None
    }
}

struct ActorRecordingSurface {
    seen: Arc<Mutex<Vec<String>>>,
}
impl EditorSurface for ActorRecordingSurface {
    fn surface_id(&self) -> &'static str {
        "mt036_actor_recorder"
    }
    fn on_selection_changed(&self, _selection: &SharedSelection) {}
    fn on_event_emitted(&self, event: &NativeEditorEvent, _emitter: &NativeEditorEventEmitter) {
        self.seen.lock().unwrap().push(event.actor_id.clone());
    }
    fn undo_local(&self) -> Option<SeamUndoResult> {
        None
    }
    fn redo_local(&self) -> Option<SeamUndoResult> {
        None
    }
}

/// A runtime-backed accepting emitter. Extension callbacks are allowed only after the ordered worker
/// queue accepts the event, so exactly-once tests must not use the explicit NoRuntime failure path.
fn accepting_emitter(ws: &str) -> (tokio::runtime::Runtime, NativeEditorEventEmitter) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("accepting callback-test runtime");
    let emitter = NativeEditorEventEmitter::new(
        ws,
        Arc::new(MockTransport::new(false)),
        Some(runtime.handle().clone()),
    );
    (runtime, emitter)
}

/// A trivial synchronous undo action (ok/ok closures) for pushing onto the bus rings.
fn sync_undo_action() -> handshake_native::undo_stack::UndoAction {
    use handshake_native::undo_stack::{UndoAction, UndoResult};
    UndoAction::sync("edit", Arc::new(UndoResult::ok), Arc::new(UndoResult::ok))
}

/// MT-036: each of the FOUR bus undo/redo choke points emits `undo_fired` exactly once with the correct
/// scope. `undo`/`redo` are `local`; `undo_cross_pane`/`redo_cross_pane` are `cross_pane` (dead-in-prod
/// before MT-036). This is the "dispatch undo via the bus (palette path)" + "dispatch undo_cross_pane"
/// exactly-once proof.
#[test]
fn undo_paths_emit_undo_fired_exactly_once_with_scope() {
    use handshake_native::interop::interaction_bus::InteractionBus;
    use handshake_native::pane_registry::PaneId;

    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut bus = InteractionBus::new();
    let (_runtime, emitter) = accepting_emitter("WS-UNDO");
    bus.set_event_emitter(emitter);
    bus.register_surface(Box::new(ScopeRecordingSurface {
        seen: Arc::clone(&seen),
    }));

    let pane: PaneId = Arc::from("pane-rich");

    // LOCAL undo + redo (the command-palette Undo/Redo AND the Ctrl+Z/Ctrl+Y chord all route through here).
    bus.push_undo_local(pane.clone(), sync_undo_action());
    assert!(
        bus.undo(&pane).is_some(),
        "undo popped the pushed local action"
    );
    assert!(
        bus.redo(&pane).is_some(),
        "redo re-applied the undone action"
    );

    // CROSS-PANE undo + redo (POLICY-2 — entirely SILENT before MT-036; scope=cross_pane).
    bus.set_focus_owner(pane.clone());
    bus.push_undo_cross_pane(sync_undo_action());
    assert!(bus.undo_cross_pane().is_some(), "cross-pane undo popped");
    assert!(
        bus.redo_cross_pane().is_some(),
        "cross-pane redo re-applied"
    );

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.as_slice(),
        &[
            ("undo_fired".to_owned(), Some("local".to_owned())),
            ("undo_fired".to_owned(), Some("local".to_owned())),
            ("undo_fired".to_owned(), Some("cross_pane".to_owned())),
            ("undo_fired".to_owned(), Some("cross_pane".to_owned())),
        ],
        "each of the four bus undo/redo choke points emits exactly ONE undo_fired with the right scope"
    );
}

/// MT-036: the command-palette `Undo` + `Undo Cross-Pane` commands (the SILENT-before-fix palette paths)
/// now emit `undo_fired` through the same central choke points. Drives the REAL registered command
/// handlers via `dispatch_command` (the exact path the palette + keybind dispatch use).
#[test]
fn command_palette_undo_commands_emit_undo_fired() {
    use handshake_native::interop::interaction_bus::{
        InteractionBus, CMD_UNDO, CMD_UNDO_CROSS_PANE,
    };
    use handshake_native::pane_registry::PaneId;

    let seen = Arc::new(Mutex::new(Vec::new()));
    let mut bus = InteractionBus::new();
    let (_runtime, emitter) = accepting_emitter("WS-CMD");
    bus.set_event_emitter(emitter);
    bus.register_surface(Box::new(ScopeRecordingSurface {
        seen: Arc::clone(&seen),
    }));
    bus.register_undo_commands();

    let ctx = egui::Context::default();
    let pane: PaneId = Arc::from("pane-code");
    bus.set_focus_owner(pane.clone());

    // The palette `Undo` command handler calls `bus.undo(focus_owner)` — SILENT before MT-036.
    bus.push_undo_local(pane.clone(), sync_undo_action());
    assert!(
        bus.dispatch_command(&ctx, CMD_UNDO),
        "the Undo command is registered + dispatched"
    );

    // The palette `Undo Cross-Pane` command handler calls `bus.undo_cross_pane()` — SILENT before MT-036.
    bus.push_undo_cross_pane(sync_undo_action());
    assert!(
        bus.dispatch_command(&ctx, CMD_UNDO_CROSS_PANE),
        "the Undo Cross-Pane command is registered + dispatched"
    );

    let seen = seen.lock().unwrap();
    assert_eq!(
        seen.as_slice(),
        &[
            ("undo_fired".to_owned(), Some("local".to_owned())),
            ("undo_fired".to_owned(), Some("cross_pane".to_owned())),
        ],
        "the command-palette Undo + Undo-Cross-Pane commands now emit undo_fired (both silent before MT-036)"
    );
}

#[test]
fn extension_callback_observes_the_same_authoritative_actor_as_the_ledger() {
    let seen = Arc::new(Mutex::new(Vec::new()));
    let (_runtime, emitter) = accepting_emitter("WS-ACTOR");
    let mut bus = handshake_native::interop::InteractionBus::new();
    bus.set_event_emitter(emitter);
    bus.register_surface(Box::new(ActorRecordingSurface {
        seen: Arc::clone(&seen),
    }));
    assert!(bus.emit_event(NativeEditorEvent::document_saved(
        "DOC-ACTOR",
        "a".repeat(64),
        "pane-rich",
        "caller-supplied-wrong-actor",
        "WS-ACTOR",
    )));
    assert_eq!(
        seen.lock().unwrap().as_slice(),
        &[handshake_native::event_emitter::DEFAULT_ACTOR_ID]
    );
}

// ── AC-8 hygiene guard (always runs) ─────────────────────────────────────────────────────────────────

#[test]
fn no_repo_local_artifact_dir() {
    assert_no_local_artifact_dir();
}

// ── AC-1/2/3 + PT-3/4/6: live native-editor ledger round-trip ────────────────────────────────────────

#[test]
fn event_emitter_native_editor_round_trip() {
    let base =
        std::env::var("HSK_TEST_BASE").unwrap_or_else(|_| "http://127.0.0.1:37501".to_owned());
    let marker = uuid::Uuid::new_v4().to_string();
    let session_id = uuid::Uuid::new_v4().to_string();
    let actor = format!("mt036-live-human-{marker}");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("integration runtime");
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(8))
        .build()
        .expect("build bounded MT-036 HTTP client");
    let (workspace, document_id, _doc_version, canvas_id, source_block_id) = runtime.block_on(async {
        let response = http
            .post(format!("{base}/workspaces"))
            .header("x-hsk-actor-id", &actor)
            .header("x-hsk-actor-kind", "human")
            .json(&serde_json::json!({"name": format!("MT036 {marker}")}))
            .send()
            .await
            .expect("create isolated workspace request");
        let status = response.status();
        let workspace_body: serde_json::Value =
            response.json().await.expect("create workspace JSON");
        assert!(status.is_success(), "create workspace -> {status}: {workspace_body}");
        let workspace = workspace_body["id"]
            .as_str()
            .expect("created workspace id")
            .to_owned();

        let identified = |request: reqwest::RequestBuilder| {
            request
                .header("x-hsk-actor-id", &actor)
                .header("x-hsk-kernel-task-run-id", "KTR-MT036-LIVE")
                .header("x-hsk-session-run-id", &session_id)
                .header("x-hsk-actor-kind", "operator")
        };
        let content = serde_json::json!({
            "type": "doc",
            "content": [{"type":"paragraph","content":[{"type":"text","text":format!("MT036 {marker}")}]}]
        });
        let response = identified(http.post(format!("{base}/knowledge/documents")))
            .json(&serde_json::json!({
                "workspace_id": &workspace,
                "title": format!("MT036 live {marker}"),
                "content_json": content,
            }))
            .send()
            .await
            .expect("create rich document request");
        let status = response.status();
        let document: serde_json::Value = response.json().await.expect("create rich document JSON");
        assert!(status.is_success(), "create rich document -> {status}: {document}");

        let response = identified(http.post(format!(
            "{base}/workspaces/{workspace}/loom/blocks"
        )))
        .json(&serde_json::json!({
            "content_type": "note",
            "title": format!("MT036 canvas source {marker}")
        }))
        .send()
        .await
        .expect("create source Loom block request");
        let status = response.status();
        let source: serde_json::Value = response.json().await.expect("source block JSON");
        assert!(status.is_success(), "create source block -> {status}: {source}");

        let response = identified(http.post(format!(
            "{base}/workspaces/{workspace}/loom/canvas-boards"
        )))
        .json(&serde_json::json!({"title": format!("MT036 canvas {marker}")}))
        .send()
        .await
        .expect("create Canvas board request");
        let status = response.status();
        let canvas: serde_json::Value = response.json().await.expect("canvas JSON");
        assert!(status.is_success(), "create Canvas -> {status}: {canvas}");

        (
            workspace,
            document["document"]["rich_document_id"]
                .as_str()
                .expect("created rich_document_id")
                .to_owned(),
            document["document"]["doc_version"]
                .as_u64()
                .expect("created doc_version"),
            canvas["block_id"]
                .as_str()
                .expect("created canvas block_id")
                .to_owned(),
            source["block_id"]
                .as_str()
                .expect("created source block_id")
                .to_owned(),
        )
    });
    use handshake_native::app::{HandshakeApp, HealthDisplayState};
    use handshake_native::backend_client::HealthInfo;
    use handshake_native::graph::canvas_board::PLACE_BLOCK_AUTHOR_ID;
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_backend_base_url_for_test(&base, runtime.handle().clone());
    app.set_active_project_id_for_test(workspace.clone());
    assert!(
        app.open_document(&document_id).opened(),
        "real shell opens the PostgreSQL-backed document in the mounted Notes pane"
    );
    let canvas_board = app.mounted_canvas_board();
    let canvas_events = app.mounted_canvas_events();
    {
        let mut board = canvas_board.lock().expect("canvas board lock");
        board.workspace_id = workspace.clone();
        board.canvas_block_id = canvas_id.clone();
    }
    assert!(
        app.dispatch_palette_action_for_test(handshake_native::command_registry::CMD_VIEW_CANVAS),
        "operator-facing View Canvas command mounts the production pane"
    );
    let captured_ctx = Arc::new(Mutex::new(None::<egui::Context>));
    let captured_ctx_ui = Arc::clone(&captured_ctx);
    let mut app_harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_state(
            move |ctx, app: &mut HandshakeApp| {
                *captured_ctx_ui.lock().expect("capture app context") = Some(ctx.clone());
                app.ui(ctx);
            },
            app,
        );
    for _ in 0..400 {
        app_harness.run_steps(1);
        let ready = canvas_board
            .lock()
            .map(|board| !board.loading && board.error.is_none())
            .unwrap_or(false);
        let rich_ready = app_harness
            .state()
            .mounted_rich_state()
            .lock()
            .map(|state| {
                state
                    .block_plain_text(0)
                    .is_some_and(|text| text.contains(&marker))
            })
            .unwrap_or(false);
        if ready
            && rich_ready
            && canvas_events
                .lock()
                .map(|events| events.is_empty())
                .unwrap_or(false)
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        canvas_board
            .lock()
            .map(|board| !board.loading && board.error.is_none())
            .unwrap_or(false),
        "mounted Canvas completed its initial backend load"
    );
    let app_ctx = captured_ctx
        .lock()
        .expect("captured app context lock")
        .clone()
        .expect("mounted app context");
    let app_bus = handshake_native::interop::InteractionBus::get_or_init(&app_ctx);
    let emitter = handshake_native::interop::InteractionBus::with_try_lock(&app_bus, |bus| {
        bus.event_emitter().cloned()
    })
    .flatten()
    .expect("production shell installed the native-editor emitter");
    assert_eq!(emitter.workspace_id(), workspace);

    // Save the real PostgreSQL document through the mounted HandshakeApp Notes pane and the production
    // File > Save dispatcher. The mounted widget drains the SaveManager completion and emits the receipt.
    // Re-open the already-mounted document through the real ShellNavigator first: mounting Canvas above
    // intentionally made Canvas active, and Save must never target an inactive editor by accident.
    assert!(
        app_harness.state_mut().open_document(&document_id).opened(),
        "real shell re-focuses the mounted Notes document before File > Save"
    );
    app_harness.run_steps(2);
    let live_ctx = app_harness.ctx.clone();
    let save_lifecycle = app_harness.state().editor_save_state_for_test(&live_ctx);
    assert!(
        app_harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(
                &live_ctx,
                handshake_native::command_registry::CMD_EDITOR_FILE_SAVE,
            ),
        "mounted app File > Save dispatch reaches the real Notes SaveManager; {save_lifecycle}"
    );
    let save_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        app_harness.run_steps(1);
        if !app_harness
            .state()
            .mounted_rich_state()
            .lock()
            .unwrap()
            .save_is_in_flight()
        {
            break;
        }
        assert!(
            std::time::Instant::now() < save_deadline,
            "timed out waiting for native rich save completion"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    assert!(
        app_harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(
                &live_ctx,
                handshake_native::command_registry::CMD_VIEW_CANVAS,
            ),
        "mounted app re-focuses the real Canvas pane before placement"
    );
    app_harness.run_steps(2);

    // Mount the real Handshake Canvas pane and click its AccessKit Place-block control. The backend
    // mints the placement; the production app drains that exact completion and emits
    // canvas_node_placed. No direct event call and no fabricated placement result is used here.
    canvas_board
        .lock()
        .expect("canvas place input lock")
        .place_block_input = source_block_id.clone();
    app_harness.run_steps(1);
    let target = app_harness
        .root()
        .children_recursive()
        .find_map(|node| {
            let access = node.accesskit_node();
            (access.author_id() == Some(PLACE_BLOCK_AUTHOR_ID)).then(|| access.id())
        })
        .expect("mounted Canvas exposes canvas.place-block");
    app_harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target,
            data: None,
        },
    ));
    let canvas_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        app_harness.run_steps(1);
        let placed = canvas_board
            .lock()
            .map(|board| {
                board
                    .placements
                    .iter()
                    .any(|placement| placement.placed_block_id == source_block_id)
            })
            .unwrap_or(false);
        if placed
            && canvas_events
                .lock()
                .map(|events| events.is_empty())
                .unwrap_or(false)
            && app_harness.state().canvas_op_cells_in_flight() == 0
        {
            break;
        }
        assert!(
            std::time::Instant::now() < canvas_deadline,
            "timed out waiting for mounted Canvas placement"
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    // The real Canvas placement completion above registered its compensating action on the production
    // cross-pane undo ring. Invoke the mounted app's Edit > Undo dispatcher against that real edit;
    // never seed a synthetic action as a substitute for the product lifecycle.
    let cross_pane_undo_count =
        handshake_native::interop::InteractionBus::with_try_lock(&app_bus, |bus| {
            bus.undo_scope().cross_pane_undo_count()
        })
        .expect("production InteractionBus available for undo inspection");
    assert_eq!(
        cross_pane_undo_count, 1,
        "the backend-confirmed Canvas placement registered exactly one compensating undo"
    );
    assert!(
        app_harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(
                &live_ctx,
                handshake_native::command_registry::CMD_EDITOR_EDIT_UNDO,
            ),
        "mounted app Edit > Undo dispatch fires the real unified undo path"
    );

    let event_ids = runtime.block_on(async {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
        let matching = loop {
            let response = reqwest::Client::builder()
                .connect_timeout(std::time::Duration::from_secs(3))
                .timeout(std::time::Duration::from_secs(8))
                .build()
                .expect("build bounded MT-036 poll client")
                .get(format!("{base}/api/flight_recorder"))
                .query(&[
                    (
                        "actor_id",
                        handshake_native::event_emitter::DEFAULT_ACTOR_ID,
                    ),
                    ("wsid", workspace.as_str()),
                    ("event_type", "system"),
                ])
                .send()
                .await
                .expect("GET flight recorder");
            assert!(
                response.status().is_success(),
                "ledger GET: {}",
                response.status()
            );
            let rows: Vec<serde_json::Value> = response.json().await.expect("ledger JSON array");
            let matching = rows
                .iter()
                .filter(|row| {
                    row["actor_id"] == handshake_native::event_emitter::DEFAULT_ACTOR_ID
                        && row["wsids"]
                            .as_array()
                            .is_some_and(|ids| ids.iter().any(|id| id == &workspace))
                })
                .cloned()
                .collect::<Vec<_>>();
            if matching.len() == 3 {
                break matching;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "timed out waiting for three ordered production-emitter events; got {matching:#?}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        };
        assert_eq!(
            matching.len(),
            3,
            "exact three correlated native events: {matching:#?}"
        );
        let actions = matching
            .iter()
            .map(|row| row["payload"]["action"].as_str().unwrap_or_default())
            .collect::<Vec<_>>();
        assert_eq!(
            actions,
            vec!["undo_fired", "canvas_node_placed", "document_saved"],
            "GET returns the three sequential writes newest-first"
        );
        let trace_id = matching[0]["trace_id"]
            .as_str()
            .expect("production emitter trace id");
        assert!(uuid::Uuid::parse_str(trace_id).is_ok());
        assert!(matching.iter().all(|row| {
            row["event_type"] == "system"
                && row["actor_id"] == handshake_native::event_emitter::DEFAULT_ACTOR_ID
                && row["trace_id"] == trace_id
                && row["session_span_id"] == trace_id
                && row["payload"]["schema_version"] == NATIVE_EDITOR_SCHEMA_VERSION
                && row["payload"]["schema"] == NATIVE_EDITOR_SCHEMA_VERSION
                && row["payload"]["workspace_id"] == workspace
        }));
        matching
            .iter()
            .map(|row| row["event_id"].as_str().unwrap().to_owned())
            .collect::<Vec<_>>()
    });

    // Open the real operator route. The mounted pane itself requests GET /flight_recorder through the
    // app driver and renders the returned rows; no test cell, parser delivery, or standalone pane is
    // substituted.
    assert!(app_harness
        .state_mut()
        .dispatch_palette_action_for_test("flightrecorder.open"));
    let pane_deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        app_harness.run_steps(1);
        let ids = author_ids(&app_harness);
        if event_ids
            .iter()
            .all(|event_id| ids.contains(&fr_event_row_author_id(event_id)))
        {
            break;
        }
        assert!(
            std::time::Instant::now() < pane_deadline,
            "mounted production Flight Recorder pane did not expose {event_ids:?}; lifecycle={:?}",
            app_harness.state().flight_recorder_state_for_test(),
        );
        std::thread::sleep(std::time::Duration::from_millis(25));
    }

    runtime.block_on(async {
        let cleanup = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .expect("build bounded MT-036 cleanup client")
            .delete(format!("{base}/workspaces/{workspace}"))
            .header("x-hsk-actor-id", &actor)
            .header("x-hsk-actor-kind", "human")
            .send()
            .await
            .expect("cleanup isolated workspace");
        assert!(cleanup.status().is_success(), "isolated workspace cleanup");
    });
}
