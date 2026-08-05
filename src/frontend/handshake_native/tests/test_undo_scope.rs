//! WP-KERNEL-012 MT-035 — the ONE unified undo scope, end-to-end against the REAL editor panes + bus.
//!
//! These tests prove the five undo policies (POLICY-1..5) and the six acceptance criteria against the
//! ACTUAL [`handshake_native::interop::InteractionBus`] + [`UnifiedUndoScope`] + the real
//! [`CodeEditorPanel`] / [`StagePane`], NOT hand-built stand-ins (the Spec-Realism Gate's "touch the
//! real Handshake-owned resource" rule). The undo-ring data structure is also unit-tested standalone in
//! `src/undo_stack.rs` (the pure, cap/local-first/no-Serialize proofs); these are the integration +
//! kittest + AccessKit proofs on top.
//!
//! AC map (honesty note after the 2026-06 adversarial harden — what is LIVE vs ADAPTER vs DEFERRED):
//! - AC-1 (POLICY-1 local-first), TWO honest proofs:
//!     * LIVE (code half): `code_pane_plain_typing_records_undo_and_shell_undo_reverts_live` and
//!       `code_pane_backspace_records_undo_and_shell_undo_reverts_live` drive the mounted app shell with
//!       real code-pane mutations. No test seeds `push_code_edit_undo`; the live producer stages the undo,
//!       the pane factory drains it into the shared bus, and shell-routed Undo restores the buffer.
//!     * LIVE (rich half): `rich_pane_ctrl_z_reverts_through_bus` drives a REAL edit + a REAL Ctrl+Z
//!       keystroke through the MOUNTED rich-editor widget harness; the doc reverts via the unified scope
//!       (the rich pane's live undo now flows through `bus.undo(pane)`, NOT a parallel `UndoManager`).
//!       The old `sync_action("rich-edit", log)` logging stand-in + its tautological assertion are GONE.
//!     * ADAPTER / data-structure (code half): `local_first_isolation_via_real_pane_adapters` +
//!       `registered_undo_command_dispatches_local_first` prove the per-pane ring + the real
//!       `push_code_edit_undo` / `push_rich_edit_undo` adapters isolate undo per pane.
//! - RISK-1 / MC-1 (500ms coalescing): `rich_undo_batcher_coalesces_rapid_keystrokes` (the batcher
//!   decision) + `rich_undo_coalesce_keeps_one_entry_reverting_the_whole_burst` (the scope-level
//!   coalesce: N rapid edits -> ONE entry that reverts the WHOLE burst, never silently dropped).
//! - AC-2 (POLICY-2 cross-pane): a route-to-stage action pushes a cross-pane undo entry; Ctrl+Shift+Z
//!   reverts the Stage pane's content to its previous value (real `StagePane`).
//! - AC-3 (POLICY-3 session-scoped): a fresh `UnifiedUndoScope` is empty AND the type cannot be
//!   serialized (a source-level guard asserting no `Serialize`/`Deserialize` derive on the scope/action).
//! - AC-4 (POLICY-4 canvas compensating): the compensating-DELETE REQUEST SHAPE against the verified
//!   MT-026 placement route is proven without a live backend. The live host drain registers a
//!   cross-pane compensating undo after a created-placement response, and the full create -> undo ->
//!   reload round-trip is proven against managed PostgreSQL whenever the integration feature is selected.
//!   V3 additionally drives the cross-pane undo through canonical Argus, proves in-flight compensation
//!   blocks reentry without reordering, focused local undo remains pane-scoped, and fresh app restart state
//!   cannot replay interrupted in-memory history.
//! - AC-5 (POLICY-5 cap): 201 pushes to a cap-200 ring -> 200; 51 to cap-50 cross-pane -> 50.
//! - AC-6 (undo-count indicator): the `render_undo_count_indicator` helper emits
//!   `undo-count-{pane_id}` with the correct count in a kittest AccessKit dump, and the live pane header
//!   reads the same shared `InteractionBus` depth in the mounted shell.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use egui_kittest::kittest::NodeT;
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
#[cfg(feature = "integration")]
#[path = "pg_proof_support/mod.rs"]
mod pg_proof_support;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use canonical_argus_driver::{ArgusObservation, CanonicalArgusDriver};
use screenshot_harness::ScreenshotHarness as Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::keymap::CodeEditorAction;
use handshake_native::code_editor::panel::CodeEditorPanel;
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::interop::interaction_bus::InteractionBus;
use handshake_native::interop::{render_undo_count_indicator, undo_count_author_id};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::stage_pane::{
    push_route_to_stage_undo, EmbedBackOutcome, StageContent, StagePane,
};
use handshake_native::undo_stack::{
    PaneUndoRing, UndoAction, UndoResult, UnifiedUndoScope, CROSS_PANE_RING_CAP, PANE_RING_CAP,
};

#[cfg(feature = "wgpu_screenshots")]
static MT035_WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(feature = "wgpu_screenshots")]
struct Mt035AsyncTaskGuard {
    active_tasks: Arc<std::sync::atomic::AtomicUsize>,
    dropped_tx: Option<std::sync::mpsc::Sender<()>>,
}

#[cfg(feature = "wgpu_screenshots")]
impl Drop for Mt035AsyncTaskGuard {
    fn drop(&mut self) {
        self.active_tasks
            .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
        if let Some(tx) = self.dropped_tx.take() {
            let _ = tx.send(());
        }
    }
}

// ── Artifact-hygiene helpers (CX-212E / CX-212F): artifacts go to the EXTERNAL root ONLY ──────────────

/// Assert NO repo-local artifact directory exists under the crate (artifact hygiene — CX-212E). Checks
/// BOTH `test_output/` AND `tests/screenshots/`; a tracked artifact under `src/` is a hygiene FAILURE.
fn assert_no_local_artifact_dir() {
    for local in [Path::new("test_output"), Path::new("tests/screenshots")] {
        assert!(
            !local.exists(),
            "no repo-local artifact dir may exist ({}) — artifacts go to the external \
             Handshake_Artifacts/handshake-test root only",
            local.display()
        );
    }
}

#[cfg(feature = "integration")]
fn mt035_proof_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-id", "mt035-live-pg")
        .header("x-hsk-actor-kind", "operator")
        .header("x-hsk-kernel-task-run-id", "WP-KERNEL-012-MT-035")
        .header("x-hsk-session-run-id", "MT-035-integration")
}

#[cfg(feature = "integration")]
fn mt035_workspace_headers(request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    request
        .header("x-hsk-actor-id", "mt035-live-pg")
        .header("x-hsk-actor-kind", "human")
}

#[cfg(feature = "integration")]
struct Mt035WorkspaceCleanup {
    base: String,
    workspace_id: String,
    armed: bool,
}

#[cfg(feature = "integration")]
impl Mt035WorkspaceCleanup {
    async fn cleanup(&mut self, client: &reqwest::Client) {
        let response = mt035_workspace_headers(
            client.delete(format!("{}/workspaces/{}", self.base, self.workspace_id)),
        )
        .send()
        .await
        .expect("delete owned MT-035 workspace");
        assert_eq!(response.status(), reqwest::StatusCode::NO_CONTENT);
        self.armed = false;
    }
}

#[cfg(feature = "integration")]
impl Drop for Mt035WorkspaceCleanup {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let base = self.base.clone();
        let workspace_id = self.workspace_id.clone();
        let _ = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("MT-035 cleanup runtime");
            runtime.block_on(async move {
                let client = reqwest::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .expect("MT-035 cleanup client");
                let _ = mt035_workspace_headers(
                    client.delete(format!("{base}/workspaces/{workspace_id}")),
                )
                .send()
                .await;
            });
        })
        .join();
    }
}

#[cfg(feature = "integration")]
async fn mt035_dispatch_created_placement(
    client: &handshake_native::backend_client::CanvasBoardClient,
    spec: handshake_native::backend_client::RequestSpec,
) -> handshake_native::backend_client::CreatedCanvasPlacement {
    let cell: handshake_native::backend_client::CanvasBoardCreateCell = Arc::new(Mutex::new(None));
    client.dispatch_created_placement(spec, Arc::clone(&cell));
    for _ in 0..600 {
        if let Some(result) = cell.lock().unwrap().take() {
            return result.expect("managed-PG placement create");
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    panic!("managed-PG placement create did not resolve within six seconds");
}

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

fn sync_action(tag: &'static str, log: Arc<Mutex<Vec<String>>>) -> UndoAction {
    let undo_log = log.clone();
    let redo_log = log;
    UndoAction::sync(
        tag,
        Arc::new(move || {
            undo_log.lock().unwrap().push(tag.to_owned());
            UndoResult::ok()
        }),
        Arc::new(move || {
            redo_log.lock().unwrap().push(format!("redo:{tag}"));
            UndoResult::ok()
        }),
    )
}

fn mt035_editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("build multi-thread runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());

    {
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
    }
    (app, runtime)
}

fn focus_code_text_surface(harness: &Harness<'_, HandshakeApp>) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_TEXT_AUTHOR_ID))
        .expect("the mounted editor.code.text TextInput node must be present");
    node.focus();
}

fn shell_indicator_value(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.value();
        }
    }
    None
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 — POLICY-1 local-first. TWO proofs, honestly separated:
//   (a) DATA-STRUCTURE / ADAPTER proof (this test + `registered_undo_command_dispatches_local_first`):
//       proves the `push_code_edit_undo` adapter + the per-pane ring isolate undo per pane. It is the
//       ring + adapter contract, not live producer behavior.
//   (b) LIVE code proofs (`code_pane_plain_typing_records_undo_and_shell_undo_reverts_live` and
//       `code_pane_backspace_records_undo_and_shell_undo_reverts_live`) drive mounted-shell code edits
//       without manual undo seeding.
//   (c) LIVE rich proof (`rich_pane_ctrl_z_reverts_through_bus`): drives a REAL edit + a REAL Ctrl+Z
//       keystroke through the mounted rich-editor widget harness and asserts the doc reverts via the
//       unified scope.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// DATA-STRUCTURE / ADAPTER proof of POLICY-1 local-first isolation (NOT a live-wiring proof — see the
/// section header). Two REAL pane adapters record onto two pane rings: `push_code_edit_undo` (code rope
/// snapshot) and `push_rich_edit_undo` (rich content_json snapshot). Undoing the focused code pane
/// reverts ONLY the code buffer; the rich pane's ring is untouched. This proves the ring + both adapters,
/// using the real `set_text` restore + the real snapshot applier — no `sync_action` logging stand-in.
#[test]
fn local_first_isolation_via_real_pane_adapters() {
    use handshake_native::rich_editor::interop_adapter::{
        push_rich_edit_undo, RichSnapshotApplier,
    };

    let code_panel = Arc::new(CodeEditorPanel::new("fn main() {}\n", "rs"));
    let code_pane = pane("pane-code");
    let rich_pane = pane("pane-rich");

    // A standalone bus (the same type the shell shares). Register the unified-undo commands.
    let mut bus = InteractionBus::new();
    bus.register_undo_commands();

    // Snapshot BEFORE the code edit, then apply a real edit to the panel.
    let before = code_panel.buffer();
    code_panel.set_text("fn main() { let x = 1; }\n");
    let after = code_panel.buffer();
    assert_ne!(
        before.to_string(),
        after.to_string(),
        "the code edit changed the buffer"
    );

    // Record the code edit on the code pane's LOCAL ring via the REAL adapter (POLICY-1).
    handshake_native::code_editor::interop_adapter::push_code_edit_undo(
        &mut bus,
        code_pane.clone(),
        &code_panel,
        before.clone(),
        after.clone(),
        "code: insert let",
    );

    // Record an UNRELATED edit on the RICH pane's ring via the REAL `push_rich_edit_undo` adapter,
    // backed by a real `Arc<Mutex<_>>` doc state + a real snapshot applier (NOT a logging stand-in).
    let rich_doc = Arc::new(Mutex::new(String::from("rich-before")));
    let restore: RichSnapshotApplier<String> = Arc::new(|s: &mut String, snap| {
        *s = snap.as_str().unwrap_or_default().to_owned();
    });
    push_rich_edit_undo(
        &mut bus,
        rich_pane.clone(),
        &rich_doc,
        serde_json::json!("rich-before"),
        serde_json::json!("rich-after"),
        restore,
        "rich: edit",
    );
    *rich_doc.lock().unwrap() = "rich-after".to_owned(); // simulate the applied edit's after-state.

    assert_eq!(bus.local_undo_count(&code_pane), 1);
    assert_eq!(bus.local_undo_count(&rich_pane), 1);

    // Focus the CODE pane and undo (local-first). Only the code buffer reverts.
    bus.set_focus_owner(code_pane.clone());
    let result = bus
        .undo(&code_pane)
        .expect("an action to undo on the focused code pane");
    assert!(result.ok, "the code undo applied: {result:?}");
    assert_eq!(
        code_panel.buffer().to_string(),
        before.to_string(),
        "POLICY-1: undoing the focused code pane restored its PRE-edit buffer"
    );
    // The rich pane's ring + doc were NOT touched (its undo_fn never fired).
    assert_eq!(
        *rich_doc.lock().unwrap(),
        "rich-after",
        "POLICY-1: the rich pane's doc was NOT reverted by the code undo (local-first isolation)"
    );
    assert_eq!(bus.local_undo_count(&code_pane), 0, "code ring drained");
    assert_eq!(
        bus.local_undo_count(&rich_pane),
        1,
        "rich ring UNTOUCHED (POLICY-1 local-first)"
    );

    // Redo re-applies the code edit.
    let redo = bus.redo(&code_pane).expect("a redo on the code pane");
    assert!(redo.ok);
    assert_eq!(
        code_panel.buffer().to_string(),
        after.to_string(),
        "redo re-applied the code edit"
    );

    // And the rich pane's OWN undo (focused) reverts ONLY the rich doc, proving the symmetric isolation
    // through the real rich adapter.
    bus.set_focus_owner(rich_pane.clone());
    let rich_result = bus.undo(&rich_pane).expect("a rich undo");
    assert!(rich_result.ok);
    assert_eq!(
        *rich_doc.lock().unwrap(),
        "rich-before",
        "the rich adapter's undo_fn restored the snapshot"
    );
}

/// The registered Ctrl+Z COMMAND (not the direct `bus.undo` call) dispatches local-first through the
/// focus owner — proving the command-bus wiring, not just the method. ADAPTER / data-structure proof
/// (the test performs the `push_code_edit_undo`); live code producer proofs are separate below.
#[test]
fn registered_undo_command_dispatches_local_first() {
    let ctx = egui::Context::default();
    let code_panel = Arc::new(CodeEditorPanel::new("abc\n", "rs"));
    let code_pane = pane("pane-code");
    let mut bus = InteractionBus::new();
    bus.register_undo_commands();

    let before = code_panel.buffer();
    code_panel.set_text("abcXYZ\n");
    let after = code_panel.buffer();
    handshake_native::code_editor::interop_adapter::push_code_edit_undo(
        &mut bus,
        code_pane.clone(),
        &code_panel,
        before.clone(),
        after,
        "edit",
    );
    bus.set_focus_owner(code_pane.clone());
    bus.push_undo_cross_pane(sync_action("cross", Arc::new(Mutex::new(Vec::new()))));

    // Dispatch the Ctrl+Z command by id (the keybind resolves to this id via matching_keybind_command).
    let ctrl_z =
        handshake_native::interop::default_keybind_for(handshake_native::interop::CMD_UNDO)
            .unwrap();
    assert_eq!(
        bus.matching_keybind_command(&ctrl_z),
        Some(handshake_native::interop::CMD_UNDO),
        "Ctrl+Z resolves to the unified undo command"
    );
    assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_UNDO));
    assert_eq!(
        code_panel.buffer().to_string(),
        before.to_string(),
        "the registered Ctrl+Z command reverted the focused code pane"
    );
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        1,
        "a local action wins and leaves cross-pane history untouched"
    );
}

#[test]
fn registered_undo_command_falls_back_to_cross_pane_when_local_empty() {
    let ctx = egui::Context::default();
    let log = Arc::new(Mutex::new(Vec::new()));
    let code_pane = pane("pane-code");
    let mut bus = InteractionBus::new();
    bus.register_undo_commands();
    bus.set_focus_owner(code_pane.clone());
    bus.push_undo_cross_pane(sync_action("cross", log.clone()));

    assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_UNDO));
    assert_eq!(
        *log.lock().unwrap(),
        vec!["cross"],
        "Ctrl+Z falls back to cross-pane history when focused local history is empty"
    );
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        0,
        "fallback consumed the cross-pane action"
    );
}

#[test]
fn registered_undo_and_redo_fall_back_to_cross_pane_without_focus_owner() {
    let ctx = egui::Context::default();
    let mut bus = InteractionBus::new();
    bus.register_undo_commands();
    bus.push_undo_cross_pane(sync_action(
        "cross-without-focus",
        Arc::new(Mutex::new(Vec::new())),
    ));

    assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_UNDO));
    assert!(
        bus.undo_scope().can_redo_cross_pane(),
        "no-owner Undo reaches the cross-pane ring"
    );
    assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_REDO));
    assert!(
        bus.undo_scope().can_undo_cross_pane(),
        "no-owner Redo reaches the cross-pane ring"
    );
}

fn assert_cross_only_surface_redo_ignores_same_pane_code_redo(
    pane_type: PaneType,
    surface: &'static str,
) {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    // One physical pane first hosted Code (leaving a local redo), then switched its active surface to
    // a cross-only surface. The PaneId intentionally stays identical: this is the production tab-switch
    // boundary that previously let hidden Code history steal Canvas/Stage redo.
    let pane_id = PaneId::from(format!("pane-code-then-{surface}"));
    app.pane_registry()
        .lock()
        .expect("registry")
        .insert(PaneRecord::new(
            pane_id.clone(),
            pane_type,
            DEFAULT_PROJECT_ID,
            Some(format!("{surface}-active")),
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    app.set_active_pane_for_test(Some(pane_id.clone()));

    let ctx = egui::Context::default();
    let bus = InteractionBus::get_or_init(&ctx);
    let log = Arc::new(Mutex::new(Vec::<String>::new()));
    let local_undo_log = log.clone();
    let local_redo_log = log.clone();
    let cross_undo_log = log.clone();
    let cross_redo_log = log.clone();
    InteractionBus::with_try_lock(&bus, |bus| {
        bus.push_undo_local(
            pane_id.clone(),
            UndoAction::sync(
                "code-local",
                Arc::new(move || {
                    local_undo_log.lock().unwrap().push("undo:code".to_owned());
                    UndoResult::ok()
                }),
                Arc::new(move || {
                    local_redo_log.lock().unwrap().push("redo:code".to_owned());
                    UndoResult::ok()
                }),
            ),
        );
        bus.set_focus_owner(pane_id.clone());
        assert!(bus.undo(&pane_id).expect("seed code redo").ok);

        bus.push_undo_cross_pane(UndoAction::sync(
            "surface-cross",
            Arc::new(move || {
                cross_undo_log.lock().unwrap().push("undo:cross".to_owned());
                UndoResult::ok()
            }),
            Arc::new(move || {
                cross_redo_log.lock().unwrap().push("redo:cross".to_owned());
                UndoResult::ok()
            }),
        ));
        assert!(bus.undo_cross_pane().expect("seed cross redo").ok);
    })
    .expect("seed shared undo scope");
    log.lock().unwrap().clear();

    assert!(app.dispatch_palette_action_for_test_with_ctx(
        &ctx,
        handshake_native::command_registry::CMD_EDITOR_EDIT_REDO,
    ));
    assert_eq!(
        *log.lock().unwrap(),
        vec!["redo:cross"],
        "active {surface} cross redo wins over stale Code local redo"
    );
    InteractionBus::with_try_lock(&bus, |bus| {
        assert!(
            bus.undo_scope().can_redo_local(&pane_id),
            "stale Code local redo remains untouched"
        );
        assert_eq!(bus.focus_owner(), Some(&pane_id));
        assert!(bus.focus_owner_is_cross_only());
        assert!(bus.undo_cross_pane().expect("seed keyboard cross redo").ok);
    })
    .expect("inspect shared undo scope");
    log.lock().unwrap().clear();

    InteractionBus::with_try_lock(&bus, |bus| {
        bus.register_undo_commands();
        assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_REDO));
        assert!(bus.undo_scope().can_redo_local(&pane_id));
    })
    .expect("dispatch registered Ctrl+Y command");
    assert_eq!(
        *log.lock().unwrap(),
        vec!["redo:cross"],
        "registered Ctrl+Y on {surface} also preserves same-pane stale Code redo"
    );
}

#[test]
fn canvas_active_menu_redo_ignores_stale_code_local_redo() {
    assert_cross_only_surface_redo_ignores_same_pane_code_redo(PaneType::AtelierEditor, "canvas");
}

#[test]
fn stage_active_menu_redo_ignores_stale_code_local_redo() {
    assert_cross_only_surface_redo_ignores_same_pane_code_redo(
        PaneType::Placeholder(handshake_native::editor_pane_factories::STAGE_PANE_LABEL.to_owned()),
        "stage",
    );
}

#[test]
fn redo_falls_back_to_cross_pane_when_local_redo_is_empty() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let code_pane = pane("pane-code");
    let mut bus = InteractionBus::new();
    bus.set_focus_owner(code_pane.clone());
    bus.push_undo_cross_pane(sync_action("cross", log.clone()));
    assert!(bus.undo_cross_pane().expect("cross undo").ok);
    assert!(bus.redo(&code_pane).expect("local-first redo fallback").ok);
    assert_eq!(*log.lock().unwrap(), vec!["cross", "redo:cross"]);
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 LIVE — the rich pane's undo flows through the unified bus scope, driven by a REAL keystroke.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Find the rich-editor surface node by its stable author_id and focus it (so `apply_frame_input` runs).
fn focus_rich_surface(harness: &Harness<'_, ()>) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("editor.rich.text"))
        .expect("the editor.rich.text interactive node must be present");
    node.focus();
}

/// AC-1 (rich half) — LIVE, through the REAL mounted rich-editor widget (NOT a stand-in). Drives a real
/// text edit + a real Ctrl+Z keystroke through the widget's per-frame input loop and asserts the document
/// reverts via the SHARED unified undo scope (POLICY-1), NOT a second per-pane `UndoManager`. This is the
/// proof the adversarial review demanded: the rich pane records its undo on the bus on a live edit, and
/// its live Ctrl+Z routes through `bus.undo(pane)` to restore the content_json snapshot. The fake
/// `sync_action("rich-edit", log)` logging stand-in + its tautological assertion were DELETED.
#[test]
fn rich_pane_ctrl_z_reverts_through_bus() {
    use handshake_native::rich_editor::document_model::node::BlockNode;
    use handshake_native::rich_editor::renderer::rich_editor_widget::{
        RichEditorState, RichEditorWidget,
    };

    // A mounted rich pane: the state carries a pane id (the production wiring point — the factory sets
    // this on mount) so its edits record + route on the bus under that pane's ring.
    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
        BlockNode::paragraph("Hello"),
    ]))));
    let rich_pane = pane("pane-rich-live");
    state.lock().unwrap().undo_pane_id = Some(rich_pane.clone());

    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 300.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    // A focused rich editor blink-repaints every frame, so `harness.run()` would exceed max_steps on the
    // never-settling caret animation. EVERY step here is a single-frame `harness.step()` (the established
    // pattern for the focused editor — see `tests/test_daily_notes.rs`).
    harness.run_steps(2);

    // The SAME shared bus the mounted widget retrieves from egui app data (so we can read the unified
    // scope's per-pane ring depth — the proof that the rich edit recorded on the bus, not a side stack).
    let bus = InteractionBus::get_or_init(&harness.ctx);

    // Focus the editor surface, then type a real character through the live input loop.
    focus_rich_surface(&harness);
    harness.step();
    let before_text = state
        .lock()
        .unwrap()
        .block_plain_text(0)
        .unwrap_or_default();
    assert_eq!(before_text, "Hello", "the doc starts as 'Hello'");

    // Drive a REAL edit: type "X" at the caret (the caret is at doc start after `new`). One frame to
    // apply, which records the undo entry on the bus.
    harness.event(egui::Event::Text("X".to_owned()));
    harness.step();
    let edited = state
        .lock()
        .unwrap()
        .block_plain_text(0)
        .unwrap_or_default();
    assert_ne!(
        edited, "Hello",
        "the typed char mutated the doc (got {edited:?})"
    );

    // PROOF the edit recorded on the UNIFIED bus scope (POLICY-1 local ring), not a parallel stack.
    let depth_after_edit =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        depth_after_edit, 1,
        "AC-1 LIVE: the rich edit recorded ONE entry on the unified bus scope (got {depth_after_edit})"
    );

    // Drive a REAL Ctrl+Z keystroke through the live loop; the widget routes it through `bus.undo(pane)`.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.step();
    harness.step(); // a second frame for the post-undo repaint to settle.

    let reverted = state
        .lock()
        .unwrap()
        .block_plain_text(0)
        .unwrap_or_default();
    assert_eq!(
        reverted, "Hello",
        "AC-1 LIVE: a real Ctrl+Z through the rich widget reverted the doc via the UNIFIED scope \
         (got {reverted:?})"
    );
    // The bus ring drained (the entry was consumed by the live undo).
    let depth_after_undo =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        depth_after_undo, 0,
        "AC-1 LIVE: the unified ring drained after the live undo (got {depth_after_undo})"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// RISK-1 / MC-1 — the RichUndoBatcher 500ms coalescing: rapid keystrokes -> ONE undo entry, not N.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The `RichUndoBatcher` coalesces rapid edits within its window into ONE undo decision: the first edit
/// pushes; subsequent edits within the window do NOT push (they coalesce into the tail). After the window
/// elapses, the next edit pushes a fresh entry. This is the RISK-1 / MC-1 contract (a burst of typing is
/// one undo, not N).
#[test]
fn rich_undo_batcher_coalesces_rapid_keystrokes() {
    use handshake_native::rich_editor::interop_adapter::{RichUndoBatcher, RICH_UNDO_BATCH_MS};
    use std::time::{Duration, Instant};

    assert_eq!(RICH_UNDO_BATCH_MS, 500, "the contract window is 500ms");
    let mut batcher = RichUndoBatcher::new();
    let t0 = Instant::now();

    // First edit ALWAYS pushes (starts a batch).
    assert!(
        batcher.should_push(t0),
        "the first edit pushes a fresh entry"
    );
    // Rapid edits within the 500ms window COALESCE (do NOT push).
    let mut pushed = 1;
    for ms in [50u64, 120, 250, 400, 499] {
        if batcher.should_push(t0 + Duration::from_millis(ms)) {
            pushed += 1;
        }
    }
    assert_eq!(
        pushed, 1,
        "RISK-1: 6 keystrokes within 500ms coalesce into ONE undo entry (got {pushed})"
    );
    // An edit AFTER the window pushes a fresh entry (a deliberate new batch).
    assert!(
        batcher.should_push(t0 + Duration::from_millis(600)),
        "an edit after the 500ms window starts a fresh undo entry"
    );
}

/// RISK-1 / MC-1 — the coalescing at the SCOPE level: a fresh-push followed by an in-window
/// replace-tail leaves ONE entry whose undo restores the BATCH-START snapshot (the whole burst reverts
/// at once), never N entries and never silently dropping the in-between edits. Uses the real
/// `push_or_coalesce_rich_edit_undo` adapter + a real `Arc<Mutex<_>>` doc state.
#[test]
fn rich_undo_coalesce_keeps_one_entry_reverting_the_whole_burst() {
    use handshake_native::rich_editor::interop_adapter::{
        push_or_coalesce_rich_edit_undo, RichSnapshotApplier,
    };

    let doc = Arc::new(Mutex::new(String::from("a")));
    let restore: RichSnapshotApplier<String> = Arc::new(|s: &mut String, snap| {
        *s = snap.as_str().unwrap_or_default().to_owned();
    });
    let mut bus = InteractionBus::new();
    let p = pane("pane-rich");

    // Edit 1 (fresh batch): "a" -> "ab". batch_before = "a".
    let pushed = push_or_coalesce_rich_edit_undo(
        &mut bus,
        p.clone(),
        &doc,
        /*should_push=*/ true,
        serde_json::json!("a"),
        serde_json::json!("a"),
        serde_json::json!("ab"),
        restore.clone(),
        "rich: edit",
    );
    assert!(pushed, "the first edit of a batch pushes a fresh entry");
    *doc.lock().unwrap() = "ab".to_owned();
    assert_eq!(bus.local_undo_count(&p), 1);

    // Edit 2 (same batch, coalesce): "ab" -> "abc". batch_before stays "a"; tail replaced.
    let pushed2 = push_or_coalesce_rich_edit_undo(
        &mut bus,
        p.clone(),
        &doc,
        /*should_push=*/ false,
        serde_json::json!("a"),
        serde_json::json!("ab"),
        serde_json::json!("abc"),
        restore.clone(),
        "rich: edit",
    );
    assert!(!pushed2, "an in-window edit coalesces (no new entry)");
    *doc.lock().unwrap() = "abc".to_owned();
    assert_eq!(
        bus.local_undo_count(&p),
        1,
        "RISK-1: the burst is STILL ONE undo entry after coalescing (not 2)"
    );

    // Edit 3 (same batch, coalesce): "abc" -> "abcd".
    push_or_coalesce_rich_edit_undo(
        &mut bus,
        p.clone(),
        &doc,
        /*should_push=*/ false,
        serde_json::json!("a"),
        serde_json::json!("abc"),
        serde_json::json!("abcd"),
        restore.clone(),
        "rich: edit",
    );
    *doc.lock().unwrap() = "abcd".to_owned();
    assert_eq!(
        bus.local_undo_count(&p),
        1,
        "still one entry after 3 coalesced edits"
    );

    // ONE undo reverts the WHOLE burst back to the batch-START snapshot "a" (not just the last char).
    bus.set_focus_owner(p.clone());
    let result = bus.undo(&p).expect("the single coalesced entry");
    assert!(result.ok);
    assert_eq!(
        *doc.lock().unwrap(),
        "a",
        "RISK-1: undoing the coalesced entry reverts the ENTIRE burst (a..abcd -> a), proving the \
         in-between edits were NOT silently dropped from history"
    );
    assert_eq!(bus.local_undo_count(&p), 0, "the ring drained");

    // Redo re-applies the burst's final state in ONE step.
    let redo = bus.redo(&p).expect("a redo");
    assert!(redo.ok);
    assert_eq!(
        *doc.lock().unwrap(),
        "abcd",
        "redo re-applies the coalesced burst's final state"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// CURRENT LIVE/DEFERRED TRUTH:
// - Code pane live typing/deletion now records local undo entries through the mounted shell.
// - Canvas placement/card creation responses now register push_canvas_placement_undo through the mounted
//   app drain after the backend-minted placement id is known.
// - The canvas compensating DELETE request shape passes below; the full create -> undo -> reload absence
//   proof is complemented by the managed-PostgreSQL integration run below.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn code_pane_plain_typing_records_undo_and_shell_undo_reverts_live() {
    let (app, _rt) = mt035_editor_shell();
    let code_panel = app.mounted_code_panel();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    let pane_id = PaneId::from("pane-a");
    let before = code_panel.buffer().to_string();
    let inserted = "// mt035 live typing undo";
    code_panel.set_single_cursor(before.len());
    focus_code_text_surface(&harness);
    harness.run_steps(1);

    harness.event(egui::Event::Text(inserted.to_owned()));
    harness.run_steps(2);

    let after = code_panel.buffer().to_string();
    assert_ne!(after, before, "the mounted code pane accepted live typing");
    assert!(
        after.ends_with(inserted),
        "the live typed text landed in the mounted code pane buffer; got tail {:?}",
        &after[after.len().saturating_sub(inserted.len())..]
    );

    let undo_depth =
        InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |b| {
            b.local_undo_count(&pane_id)
        })
        .expect("bus lock");
    assert_eq!(
        undo_depth, 1,
        "AC-1 LIVE: plain code typing records one local undo entry without manual seeding"
    );

    code_panel.request_undo_for_test();
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "AC-1 LIVE: shell-routed Undo reverted the live typed code edit"
    );

    let undo_depth_after =
        InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |b| {
            b.local_undo_count(&pane_id)
        })
        .expect("bus lock");
    assert_eq!(
        undo_depth_after, 0,
        "AC-1 LIVE: the code pane local undo ring drained after shell Undo"
    );
}

#[test]
fn code_pane_backspace_records_undo_and_shell_undo_reverts_live() {
    let (app, _rt) = mt035_editor_shell();
    let code_panel = app.mounted_code_panel();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    let pane_id = PaneId::from("pane-a");
    let before = "fn main() {}\n// delete-target";
    code_panel.set_text(before);
    code_panel.set_single_cursor(before.len());
    focus_code_text_surface(&harness);
    harness.run_steps(1);

    harness.key_press(egui::Key::Backspace);
    harness.run_steps(2);

    let after = code_panel.buffer().to_string();
    assert_ne!(after, before, "Backspace changed the mounted code pane");
    assert!(
        after.ends_with("delete-targe"),
        "Backspace removed the final byte-grapheme from the mounted code pane; got {after:?}"
    );

    let undo_depth =
        InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |b| {
            b.local_undo_count(&pane_id)
        })
        .expect("bus lock");
    assert_eq!(
        undo_depth, 1,
        "AC-1 LIVE: Backspace records one local undo entry without manual seeding"
    );

    code_panel.request_undo_for_test();
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "AC-1 LIVE: shell-routed Undo reverted the live Backspace edit"
    );
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_external_artifact_dir(subdir: &str) -> PathBuf {
    let approved_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(4)
        .expect("handshake_native manifest is nested below the Handshake Worktrees root")
        .join("Handshake_Artifacts");
    let root = std::env::var_os("HANDSHAKE_ARTIFACTS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| approved_root.clone());
    assert!(root.is_absolute());
    assert_eq!(root, approved_root);
    root.join("handshake-test").join(subdir)
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_sha256_file(path: &Path) -> String {
    use sha2::Digest as _;
    format!(
        "{:x}",
        sha2::Sha256::digest(std::fs::read(path).expect("read MT-035 proof artifact"))
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_sha256_json(value: &serde_json::Value) -> String {
    use sha2::Digest as _;
    format!(
        "{:x}",
        sha2::Sha256::digest(
            serde_json::to_vec(value).expect("serialize MT-035 Argus tree for SHA-256")
        )
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_source_candidate_identity() -> (String, serde_json::Value) {
    use sha2::Digest as _;

    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .expect("handshake_native manifest is nested below the repository root");
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .expect("resolve MT-035 proof HEAD");
    assert!(head.status.success());
    let head_sha = String::from_utf8(head.stdout)
        .expect("HEAD is UTF-8")
        .trim()
        .to_owned();
    let diff = std::process::Command::new("git")
        .args(["diff", "--binary", "HEAD", "--", "."])
        .current_dir(repo_root)
        .output()
        .expect("read MT-035 source-candidate diff");
    assert!(diff.status.success());
    let diff_sha256 = format!("{:x}", sha2::Sha256::digest(&diff.stdout));
    let identity = format!("{head_sha}-worktree-{}", &diff_sha256[..16]);
    (
        identity.clone(),
        serde_json::json!({
            "identity": identity,
            "head_sha": head_sha,
            "tracked_diff_sha256": diff_sha256,
            "tracked_diff_bytes": diff.stdout.len()
        }),
    )
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_node_by_author_id<'a>(
    value: &'a serde_json::Value,
    author_id: &str,
) -> Option<&'a serde_json::Value> {
    match value {
        serde_json::Value::Object(map) => {
            if map.get("author_id").and_then(serde_json::Value::as_str) == Some(author_id) {
                return Some(value);
            }
            map.values()
                .find_map(|child| mt035_node_by_author_id(child, author_id))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|child| mt035_node_by_author_id(child, author_id)),
        _ => None,
    }
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_node_json_value(tree: &serde_json::Value, author_id: &str) -> Option<serde_json::Value> {
    let raw = mt035_node_by_author_id(tree, author_id)?["value"].as_str()?;
    serde_json::from_str(raw).ok()
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_node_value(tree: &serde_json::Value, author_id: &str) -> Option<String> {
    mt035_node_by_author_id(tree, author_id)?["value"]
        .as_str()
        .map(ToOwned::to_owned)
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_node_is_enabled(tree: &serde_json::Value, author_id: &str) -> bool {
    mt035_node_by_author_id(tree, author_id)
        .is_some_and(|node| node["disabled"].as_bool() == Some(false))
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_pending_observer_author_ids(tree: &serde_json::Value) -> Vec<String> {
    fn visit(value: &serde_json::Value, pending: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                let author_id = map.get("author_id").and_then(serde_json::Value::as_str);
                let observer_pending = author_id.is_some_and(|id| {
                    id.ends_with(".argus-action-completion")
                        && map
                            .get("value")
                            .and_then(serde_json::Value::as_str)
                            .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok())
                            .is_some_and(|token| token["state"] == "pending")
                });
                if observer_pending {
                    pending.push(author_id.expect("checked observer author id").to_owned());
                }
                for child in map.values() {
                    visit(child, pending);
                }
            }
            serde_json::Value::Array(values) => {
                for child in values {
                    visit(child, pending);
                }
            }
            _ => {}
        }
    }

    let mut pending = Vec::new();
    visit(tree, &mut pending);
    pending.sort();
    pending.dedup();
    pending
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_live_has_author_id(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> bool {
    harness
        .root()
        .children_recursive()
        .any(|node| node.accesskit_node().author_id() == Some(author_id))
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_live_json_value(
    harness: &Harness<'_, HandshakeApp>,
    author_id: &str,
) -> Option<serde_json::Value> {
    serde_json::from_str(&shell_indicator_value(harness, author_id)?).ok()
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_terminal_detail(tree: &serde_json::Value, author_id: &str) -> Option<serde_json::Value> {
    let token = mt035_node_json_value(tree, author_id)?;
    let raw = token["terminal_detail"].as_str()?;
    serde_json::from_str(raw).ok()
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_action_proof(
    target: &str,
    observation: &ArgusObservation,
    predicate_id: &str,
) -> serde_json::Value {
    let receipt = observation.after["action_receipts"]
        .as_array()
        .and_then(|receipts| {
            receipts
                .iter()
                .find(|receipt| receipt["receipt_id"].as_u64() == Some(observation.receipt_id))
        })
        .unwrap_or_else(|| {
            panic!(
                "MT-035 action {target} retains receipt {}",
                observation.receipt_id
            )
        });
    assert!(matches!(
        observation.receipt_status.as_str(),
        "applied" | "rejected"
    ));
    assert!(observation.terminal_refreshed);
    let predicate = observation
        .terminal_predicates
        .iter()
        .find(|predicate| predicate.predicate_id == predicate_id)
        .unwrap_or_else(|| panic!("MT-035 action {target} retains predicate {predicate_id}"));
    assert!(predicate.passed);
    let completion_token = receipt["observed_value"]
        .as_str()
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
    let terminal_detail = completion_token
        .as_ref()
        .and_then(|token| token["terminal_detail"].as_str())
        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
    let requested_command_id = match target {
        "menu-edit" => "menu.edit.open",
        "menu.edit.undo-cross-pane" => {
            handshake_native::command_registry::CMD_EDITOR_EDIT_UNDO_CROSS_PANE
        }
        "menu.edit.undo" => handshake_native::command_registry::CMD_EDITOR_EDIT_UNDO,
        _ => target,
    };
    serde_json::json!({
        "requested_action": "argus.click",
        "requested_command_id": requested_command_id,
        "stable_author_id": target,
        "binding_identity": observation.agent_id,
        "receipt": receipt,
        "completion_token": completion_token,
        "terminal_detail": terminal_detail,
        "correlation_id": observation.correlation_id,
        "compensation_action_id": terminal_detail
            .as_ref()
            .and_then(|detail| detail["action_id"].as_str()),
        "before_tree": observation.before,
        "after_tree": observation.after,
        "before_tree_sha256": mt035_sha256_json(&observation.before),
        "after_tree_sha256": mt035_sha256_json(&observation.after),
        "receipt_id": observation.receipt_id,
        "receipt_status": observation.receipt_status,
        "terminal_observed_sequence": observation.terminal_observed_sequence,
        "target_selected_before": observation.target_selected_before,
        "target_selected_after": observation.target_selected_after,
        "terminal_predicate": predicate
    })
}

#[cfg(feature = "wgpu_screenshots")]
fn mt035_capture_frame(
    harness: &mut Harness<'_, HandshakeApp>,
    proof_dir: &Path,
    filename: &str,
) -> serde_json::Value {
    let path = proof_dir.join(filename);
    harness.step();
    let image = harness.render().expect("MT-035 mounted WGPU proof frame");
    let dimensions = [image.width(), image.height()];
    assert_eq!(
        dimensions,
        [1400, 900],
        "MT-035 proof frame pixels must match the declared mounted viewport"
    );
    image.save(&path).expect("save mounted MT-035 proof frame");
    serde_json::json!({
        "path": path,
        "sha256": mt035_sha256_file(&path),
        "dimensions": dimensions,
        "viewport": [1400, 900],
        "capture_method": "mounted_wgpu_harness"
    })
}

fn mt035_bus_counts(ctx: &egui::Context, pane_id: &PaneId) -> (usize, usize, bool) {
    let bus = InteractionBus::get_or_init(ctx);
    InteractionBus::with_try_lock(&bus, |bus| {
        (
            bus.local_undo_count(pane_id),
            bus.undo_scope().cross_pane_undo_count(),
            bus.undo_scope().cross_pane_async_pending(),
        )
    })
    .expect("read MT-035 bus counts")
}

#[test]
fn mounted_replace_all_batch_is_one_ctrl_z_step_and_restarts_continuation_after_undo() {
    let (app, _rt) = mt035_editor_shell();
    let code_panel = app.mounted_code_panel();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    let count = handshake_native::code_editor::REPLACE_ALL_CAP + 25;
    let before = "x ".repeat(count);
    code_panel.set_text(&before);
    code_panel.open_find(true);
    code_panel.set_find_query("x");
    code_panel.set_replace_text("y");
    assert_eq!(
        code_panel.replace_all(),
        handshake_native::code_editor::REPLACE_ALL_CAP
    );
    harness.run_steps(2);

    let pane_id = PaneId::from("pane-a");
    let undo_depth =
        InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |bus| {
            bus.local_undo_count(&pane_id)
        })
        .expect("inspect mounted Replace All undo depth");
    assert_eq!(
        undo_depth, 1,
        "one effective Replace All batch stages exactly one unified undo snapshot"
    );
    assert_eq!(
        code_panel
            .find_state()
            .expect("replace bar remains mounted")
            .replace_all_remaining,
        25
    );

    focus_code_text_surface(&harness);
    harness.run_steps(1);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "mounted Ctrl+Z reverts the complete Replace All batch in one step"
    );

    // Undo changes the buffer version. A subsequent batch must discard the pre-undo continuation,
    // re-search the restored document, and expose the same bounded remainder from a fresh plan.
    assert_eq!(
        code_panel.replace_all(),
        handshake_native::code_editor::REPLACE_ALL_CAP
    );
    harness.run_steps(2);
    assert_eq!(
        code_panel
            .find_state()
            .expect("replace bar remains mounted after fresh batch")
            .replace_all_remaining,
        25,
        "post-undo Replace All restarted consistently from the restored live document"
    );
}

#[test]
fn settings_overlay_keeps_ctrl_z_from_mutating_editor_history() {
    let (mut app, _rt) = mt035_editor_shell();
    app.open_settings();
    let pane_id = PaneId::from("pane-a");
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |bus| {
        bus.set_focus_owner(pane_id.clone());
        bus.push_undo_local(
            pane_id.clone(),
            sync_action("must-remain-while-settings-open", log.clone()),
        );
    })
    .expect("seed editor undo while Settings owns keyboard input");

    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run_steps(2);

    assert!(
        harness.state().settings_open(),
        "Settings remains the active overlay"
    );
    assert!(
        log.lock().unwrap().is_empty(),
        "Settings Ctrl+Z did not invoke editor undo"
    );
    let depth = InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |bus| {
        bus.local_undo_count(&pane_id)
    })
    .expect("inspect editor undo after Settings Ctrl+Z");
    assert_eq!(depth, 1, "Settings Ctrl+Z leaves editor history untouched");
}

#[test]
fn canvas_created_placement_response_registers_cross_pane_undo_in_live_shell() {
    let (mut app, _rt) = mt035_editor_shell();
    app.deliver_canvas_created_placement_for_test(
        "ws-mt035",
        "canvas-mt035",
        handshake_native::backend_client::CreatedCanvasPlacement {
            placement_id: "LCP-mt035".to_owned(),
            placed_block_id: "blk-mt035".to_owned(),
            x: 40.0,
            y: 50.0,
            w: 200.0,
            h: 120.0,
            created_by_request: true,
        },
        "canvas: place block",
        false, // WP-KERNEL-012 MT-080 FIX A: a place-block reference, not a free-text card.
    )
    .expect("inject created-placement result into mounted host drain");

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let cross_depth =
        InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |b| {
            b.undo_scope().cross_pane_undo_count()
        })
        .expect("bus lock");
    assert_eq!(
        cross_depth, 1,
        "AC-4 LIVE: a created canvas placement response registers one cross-pane compensating undo entry"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-2 — POLICY-2 cross-pane: route-to-stage + Ctrl+Shift+Z reverts the REAL StagePane content.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn ctrl_shift_z_reverts_route_to_stage() {
    let ctx = egui::Context::default();
    let stage = Arc::new(Mutex::new(StagePane::new()));
    let mut bus = InteractionBus::new();
    bus.register_undo_commands();

    // BEFORE: the stage already holds a prior correlated route. Route a new selection, then record the
    // complete snapshots so undo/redo proves causal identity cannot drift away from its content.
    stage.lock().unwrap().set_content_correlated(
        StageContent::Selection("before".to_owned(), "DOC-before".to_owned()),
        Some("cause-before".to_owned()),
    );
    // A post-save ledger receipt may be pending when this route snapshot is taken. Route history must
    // not capture that async status: after the shell acknowledges/drops its exact receipt, undo/redo
    // cannot fabricate a retry surface that would start a new capture.
    stage.lock().unwrap().last_embed_back = Some(EmbedBackOutcome::LedgerPending {
        artifact_id: "artifact-before".to_owned(),
        sha256: "a".repeat(64),
        target_pane: "pane-before".to_owned(),
        event_id: "event-before".to_owned(),
        error: "pending acknowledgement".to_owned(),
    });
    let previous = stage.lock().unwrap().route_snapshot();
    let routed = StageContent::Selection("hello".to_owned(), "DOC-7".to_owned());
    stage
        .lock()
        .unwrap()
        .set_content_correlated(routed.clone(), Some("cause-next".to_owned()));
    let next = stage.lock().unwrap().route_snapshot();
    push_route_to_stage_undo(&mut bus, &stage, previous, next, "route to stage");

    assert_eq!(
        stage.lock().unwrap().content,
        routed,
        "the stage shows the routed selection"
    );
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        1,
        "one cross-pane action recorded"
    );

    // Ctrl+Shift+Z restores both content and the prior causal identity.
    let ctrl_shift_z = handshake_native::interop::default_keybind_for(
        handshake_native::interop::CMD_UNDO_CROSS_PANE,
    )
    .unwrap();
    assert_eq!(
        bus.matching_keybind_command(&ctrl_shift_z),
        Some(handshake_native::interop::CMD_UNDO_CROSS_PANE)
    );
    assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_UNDO_CROSS_PANE));
    {
        let stage = stage.lock().unwrap();
        assert_eq!(
            stage.content,
            StageContent::Selection("before".to_owned(), "DOC-before".to_owned()),
            "AC-2: Ctrl+Shift+Z restored the exact prior Stage route"
        );
        assert_eq!(stage.causal_action_id.as_deref(), Some("cause-before"));
        assert!(
            stage.last_embed_back.is_none(),
            "undo must not time-travel LedgerPending without the shell-owned exact receipt"
        );
    }
    // Redo re-routes it.
    assert!(bus.redo_cross_pane().is_some());
    {
        let stage = stage.lock().unwrap();
        assert_eq!(
            stage.content, routed,
            "cross-pane redo re-routed the selection"
        );
        assert_eq!(stage.causal_action_id.as_deref(), Some("cause-next"));
        assert!(
            stage.last_embed_back.is_none(),
            "redo must not fabricate an async LedgerPending status"
        );
    }
}

/// Cross-pane undo is INDEPENDENT of any pane's local-first ring: a focused pane with its OWN local
/// undo does not consume the cross-pane entry, and Ctrl+Z (local-first) does not fire the cross-pane
/// action while the focused pane has local actions.
#[test]
fn cross_pane_ring_is_independent_of_local_rings() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut bus = InteractionBus::new();
    let code_pane = pane("pane-code");
    bus.push_undo_local(code_pane.clone(), sync_action("local", log.clone()));
    bus.push_undo_cross_pane(sync_action("cross", log.clone()));
    bus.set_focus_owner(code_pane.clone());

    // Local-first undo consumes the LOCAL action, not the cross-pane one.
    bus.undo(&code_pane).unwrap();
    assert_eq!(*log.lock().unwrap(), vec!["local"]);
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        1,
        "cross-pane entry survived a local undo"
    );
    // The cross-pane undo consumes the cross-pane action.
    bus.undo_cross_pane().unwrap();
    assert_eq!(*log.lock().unwrap(), vec!["local", "cross"]);
}

#[test]
fn async_cross_pane_undo_without_runtime_does_not_advance_history() {
    let mut bus = InteractionBus::new();
    bus.push_undo_cross_pane(UndoAction::async_compensating(
        "canvas-no-runtime",
        "canvas placement",
        Arc::new(UndoResult::ok),
        Arc::new(UndoResult::ok),
        Arc::new(|| Box::pin(async { UndoResult::ok() })),
        Arc::new(|| Box::pin(async { UndoResult::ok() })),
    ));

    let result = bus
        .undo_cross_pane()
        .expect("the cross-pane action reports the missing runtime");
    assert!(!result.ok, "missing runtime is a typed failure");
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        1,
        "failed async dispatch does not move the action to redo history"
    );
}

#[test]
fn backend_touching_cross_pane_transitions_are_serialized_until_reconciled() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("parked runtime");
    let mut bus = InteractionBus::new();
    bus.set_undo_runtime(runtime.handle().clone());
    let pending_async: handshake_native::undo_stack::UndoAsyncFn = Arc::new(|| {
        Box::pin(async {
            std::future::pending::<()>().await;
            UndoResult::ok()
        })
    });
    for action_id in ["canvas-serial-1", "canvas-serial-2"] {
        bus.push_undo_cross_pane(UndoAction::async_compensating(
            action_id,
            action_id,
            Arc::new(UndoResult::ok),
            Arc::new(UndoResult::ok),
            Arc::clone(&pending_async),
            Arc::clone(&pending_async),
        ));
    }

    let first = bus.undo_cross_pane().expect("first async undo dispatches");
    assert!(first.ok && first.error.is_none());
    let blocked = bus
        .undo_cross_pane()
        .expect("second input receives a typed in-flight result");
    assert!(!blocked.ok);
    assert!(blocked
        .error
        .as_deref()
        .is_some_and(|error| error.contains("already in flight")));
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        1,
        "the second action remains in authoritative undo order until the first reconciles"
    );
}

#[test]
#[cfg(feature = "wgpu_screenshots")]
fn mt035_v4_canonical_argus_proves_undo_interruption_settlement_and_mounted_restart_empty() {
    use handshake_native::interop::interaction_bus::UndoTransitionOperation;
    use handshake_native::undo_stack::AsyncUndoDirection;

    let _wgpu_guard = MT035_WGPU_SERIAL_GUARD
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let run_id = format!(
        "mt035-v4-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after epoch")
            .as_nanos()
    );
    let proof_dir = mt035_external_artifact_dir(&format!("wp-kernel-012-mt-035-v4/{run_id}"));
    assert!(
        !proof_dir.exists(),
        "MT-035 V4 run directory must be unique"
    );
    std::fs::create_dir_all(&proof_dir).expect("create unique MT-035 V4 artifact directory");
    let (source_candidate_id, source_candidate) = mt035_source_candidate_identity();
    const COMPENSATION_ACTION_ID: &str = "mt035-v4-canvas-compensation";

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("MT-035 V4 mounted runtime");
    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let ctx = harness.ctx.clone();
    assert!(
        harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(&ctx, "view.code-editor"),
        "open the mounted code editor through the product command route"
    );
    harness.run_steps(4);
    focus_code_text_surface(&harness);
    let code_pane = harness
        .state()
        .active_pane()
        .cloned()
        .expect("the mounted code editor owns the active pane");

    let local_log = Arc::new(Mutex::new(Vec::<String>::new()));
    let (release_tx, release_rx) = tokio::sync::oneshot::channel::<()>();
    let release_rx = Arc::new(Mutex::new(Some(release_rx)));
    let active_compensation_tasks = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (future_dropped_tx, future_dropped_rx) = std::sync::mpsc::channel::<()>();
    let pending_async: handshake_native::undo_stack::UndoAsyncFn = {
        let release_rx = Arc::clone(&release_rx);
        let active_compensation_tasks = Arc::clone(&active_compensation_tasks);
        Arc::new(move || {
            let release_rx = release_rx
                .lock()
                .expect("lock MT-035 async release receiver")
                .take()
                .expect("MT-035 compensation future is dispatched once");
            active_compensation_tasks.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let task_guard = Mt035AsyncTaskGuard {
                active_tasks: Arc::clone(&active_compensation_tasks),
                dropped_tx: Some(future_dropped_tx.clone()),
            };
            Box::pin(async move {
                let _task_guard = task_guard;
                release_rx
                    .await
                    .expect("MT-035 compensation release sender remains live");
                UndoResult::ok()
            })
        })
    };
    let safe_redo_async: handshake_native::undo_stack::UndoAsyncFn =
        Arc::new(|| Box::pin(async { UndoResult::ok() }));
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        InteractionBus::with_try_lock(&bus, |bus| {
            bus.set_undo_runtime(runtime.handle().clone());
            bus.register_undo_commands();
            bus.set_focus_owner(code_pane.clone());
            bus.push_undo_local(
                code_pane.clone(),
                sync_action("focused-code-local", local_log.clone()),
            );
            bus.push_undo_cross_pane(UndoAction::async_compensating(
                COMPENSATION_ACTION_ID,
                "canvas: interrupted placement compensation",
                Arc::new(UndoResult::ok),
                Arc::new(UndoResult::ok),
                pending_async,
                safe_redo_async,
            ));
        })
        .expect("seed MT-035 V4 undo state");
    }
    assert_eq!(
        mt035_bus_counts(&harness.ctx, &code_pane),
        (1, 1, false),
        "pre-Argus state has one focused local entry and one cross-pane compensation entry"
    );

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "mt035-v4-undo-interruption");
    let initial_tree = argus.inspect(&mut harness);
    let indicator_author_id = undo_count_author_id(code_pane.as_ref());
    assert_eq!(
        mt035_node_value(&initial_tree, &indicator_author_id).as_deref(),
        Some("Undo (1)"),
        "the mounted focused code pane exposes its exact initial local undo depth"
    );
    let initial_frame = mt035_capture_frame(&mut harness, &proof_dir, "01-before.png");
    assert_eq!(
        shell_indicator_value(&harness, &indicator_author_id).as_deref(),
        Some("Undo (1)"),
        "before frame is captured from the live local-depth-1 state"
    );

    let menu_open = argus.click_expect_applied_and_reinspect(&mut harness, "menu-edit");
    assert!(
        mt035_node_by_author_id(&menu_open.before, "menu.edit.undo-cross-pane").is_none(),
        "Edit leaf is not stale before opening the dropdown"
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "edit-menu-freshly-open-with-enabled-undo-leaves",
        serde_json::json!({
            "required_leaf": "menu.edit.undo-cross-pane",
            "focused_pane_id": code_pane.as_ref(),
        }),
        |tree| {
            mt035_node_is_enabled(tree, "menu.edit.undo-cross-pane")
                && mt035_node_is_enabled(tree, "menu.edit.undo")
        },
    );
    let menu_open = argus.latest_terminal_observation();
    let menu_open_frame = mt035_capture_frame(&mut harness, &proof_dir, "02-edit-open.png");
    assert!(
        mt035_live_has_author_id(&harness, "menu.edit.undo-cross-pane"),
        "Edit-open frame is captured only while its cross-pane Undo leaf is live"
    );

    argus.click_expect_applied_and_reinspect(&mut harness, "menu.edit.undo-cross-pane");
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "cross-pane-undo-is-exactly-pending-and-local-depth-is-unchanged",
        serde_json::json!({
            "compensation_action_id": COMPENSATION_ACTION_ID,
            "focused_pane_id": code_pane.as_ref(),
            "expected_local_before": 1,
            "expected_local_after": 1,
        }),
        |tree| {
            let detail = mt035_terminal_detail(
                tree,
                handshake_native::app::MT035_ARGUS_ACTION_COMPLETION_AUTHOR_ID,
            );
            let state =
                mt035_node_json_value(tree, handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID);
            detail.as_ref().is_some_and(|detail| {
                detail["operation"] == "cross_pane_undo"
                    && detail["action_id"] == COMPENSATION_ACTION_ID
                    && detail["focused_pane_id"] == code_pane.as_ref()
                    && detail["local_before"] == 1
                    && detail["local_after"] == 1
                    && detail["pending"] == true
                    && detail["pending_action_id"] == COMPENSATION_ACTION_ID
            }) && state.as_ref().is_some_and(|state| {
                state["pending"] == true
                    && state["pending_action_id"] == COMPENSATION_ACTION_ID
                    && state["focused_local_count"] == 1
                    && state["cross_undo_count"] == 0
            })
        },
    );
    let cross_undo = argus.latest_terminal_observation();
    assert_eq!(
        mt035_bus_counts(&harness.ctx, &code_pane),
        (1, 0, true),
        "Argus menu action dispatched the async cross-pane undo and left focused local undo intact"
    );
    let cross_pending_frame =
        mt035_capture_frame(&mut harness, &proof_dir, "03-cross-pane-pending.png");
    let live_cross_pending =
        mt035_live_json_value(&harness, handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID)
            .expect("cross-pending frame has structured live status");
    assert_eq!(live_cross_pending["pending"], true);
    assert_eq!(
        live_cross_pending["pending_action_id"],
        COMPENSATION_ACTION_ID
    );

    let retry_menu = argus.click_expect_applied_and_reinspect(&mut harness, "menu-edit");
    assert!(
        mt035_node_by_author_id(&retry_menu.before, "menu.edit.undo-cross-pane").is_none(),
        "the cross-pane leaf closed after the preceding action"
    );
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "edit-menu-reopened-for-blocked-retry",
        |tree| mt035_node_is_enabled(tree, "menu.edit.undo-cross-pane"),
    );
    let retry_menu = argus.latest_terminal_observation();

    argus.click_expect_typed_rejected_and_reinspect(
        &mut harness,
        "menu.edit.undo-cross-pane",
        "already in flight",
    );
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "blocked-retry-is-typed-and-keeps-the-same-pending-operation",
        serde_json::json!({
            "compensation_action_id": COMPENSATION_ACTION_ID,
            "expected_error_fragment": "already in flight",
        }),
        |tree| {
            let detail = mt035_terminal_detail(
                tree,
                handshake_native::app::MT035_ARGUS_ACTION_COMPLETION_AUTHOR_ID,
            );
            let state =
                mt035_node_json_value(tree, handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID);
            detail.as_ref().is_some_and(|detail| {
                detail["operation"] == "cross_pane_undo"
                    && detail["action_id"] == COMPENSATION_ACTION_ID
                    && detail["pending"] == true
                    && detail["pending_action_id"] == COMPENSATION_ACTION_ID
                    && detail["error"]
                        .as_str()
                        .is_some_and(|error| error.contains("already in flight"))
            }) && state.as_ref().is_some_and(|state| {
                state["pending"] == true
                    && state["pending_action_id"] == COMPENSATION_ACTION_ID
                    && state["focused_local_count"] == 1
            })
        },
    );
    let blocked = argus.latest_terminal_observation();
    assert_eq!(
        mt035_bus_counts(&harness.ctx, &code_pane),
        (1, 0, true),
        "typed blocked retry cannot mutate local depth or pending compensation identity"
    );

    let local_menu = argus.click_expect_applied_and_reinspect(&mut harness, "menu-edit");
    assert!(
        mt035_node_by_author_id(&local_menu.before, "menu.edit.undo").is_none(),
        "the local Undo leaf closed after the preceding action"
    );
    argus.assert_latest_terminal_predicate(
        &mut harness,
        "edit-menu-reopened-for-independent-local-undo",
        |tree| mt035_node_is_enabled(tree, "menu.edit.undo"),
    );
    let local_menu = argus.latest_terminal_observation();

    argus.click_expect_applied_and_reinspect(&mut harness, "menu.edit.undo");
    argus.assert_latest_terminal_predicate_with_evidence(
        &mut harness,
        "focused-local-undo-decrements-only-local-depth-and-keeps-pending-identity",
        serde_json::json!({
            "compensation_action_id": COMPENSATION_ACTION_ID,
            "focused_pane_id": code_pane.as_ref(),
            "expected_local_before": 1,
            "expected_local_after": 0,
        }),
        |tree| {
            let detail = mt035_terminal_detail(
                tree,
                handshake_native::app::MT035_ARGUS_ACTION_COMPLETION_AUTHOR_ID,
            );
            let state =
                mt035_node_json_value(tree, handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID);
            detail.as_ref().is_some_and(|detail| {
                detail["operation"] == "local_undo"
                    && detail["focused_pane_id"] == code_pane.as_ref()
                    && detail["local_before"] == 1
                    && detail["local_after"] == 0
                    && detail["pending_action_id"] == COMPENSATION_ACTION_ID
            }) && state.as_ref().is_some_and(|state| {
                state["pending"] == true
                    && state["pending_action_id"] == COMPENSATION_ACTION_ID
                    && state["focused_local_count"] == 0
            })
        },
    );
    let local = argus.latest_terminal_observation();
    assert_eq!(
        local_log.lock().unwrap().as_slice(),
        ["focused-code-local"],
        "the focused local action fired exactly once"
    );
    assert_eq!(
        mt035_bus_counts(&harness.ctx, &code_pane),
        (0, 0, true),
        "local focused history drains independently while the interrupted compensation remains pending"
    );
    let local_pending_frame = mt035_capture_frame(
        &mut harness,
        &proof_dir,
        "04-local-undone-cross-pending.png",
    );
    assert_eq!(
        shell_indicator_value(&harness, &indicator_author_id).as_deref(),
        Some("Undo (0)"),
        "local-undone frame is captured from the live local-depth-0 state"
    );
    assert_eq!(
        mt035_live_json_value(&harness, handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID,)
            .expect("local-undone frame has structured live status")["pending"],
        true
    );

    release_tx
        .send(())
        .expect("release the exact MT-035 async compensation task");
    future_dropped_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("the spawned compensation future is dropped after terminal completion");
    assert_eq!(
        active_compensation_tasks.load(std::sync::atomic::Ordering::SeqCst),
        0,
        "no compensation future remains active before canonical settlement"
    );
    harness.state().deliver_canvas_compensation_for_test(
        COMPENSATION_ACTION_ID,
        AsyncUndoDirection::Undo,
        "mt035-v4-proof-workspace",
        "mt035-v4-proof-canvas",
        Ok(()),
    );
    harness.run_steps(3);

    let settlement_tree = argus.inspect(&mut harness);
    let settlement_state = mt035_node_json_value(
        &settlement_tree,
        handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID,
    )
    .expect("settlement state is persistently observable");
    let settlement_pending_observers = mt035_pending_observer_author_ids(&settlement_tree);
    assert!(
        settlement_pending_observers.is_empty(),
        "strict settlement cannot retain any pending click-completion observer: {settlement_pending_observers:?}"
    );
    assert_eq!(settlement_state["pending"], false);
    assert_eq!(
        settlement_state["pending_action_id"],
        serde_json::Value::Null
    );
    assert_eq!(settlement_state["last_operation"], "compensation_settled");
    assert_eq!(settlement_state["last_action_id"], COMPENSATION_ACTION_ID);
    assert_eq!(
        mt035_bus_counts(&harness.ctx, &code_pane),
        (0, 0, false),
        "canonical completion drain settles the exact compensation with no residual history"
    );
    let settlement_transition =
        InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |bus| {
            bus.last_undo_transition().cloned()
        })
        .expect("inspect settlement transition")
        .expect("settlement transition exists");
    assert_eq!(
        settlement_transition.operation,
        UndoTransitionOperation::CompensationSettled
    );
    assert_eq!(
        settlement_transition.action_id.as_deref(),
        Some(COMPENSATION_ACTION_ID)
    );
    assert!(settlement_transition.pending_after.is_none());
    let settlement_frame =
        mt035_capture_frame(&mut harness, &proof_dir, "05-compensation-settled.png");
    assert_eq!(
        mt035_live_json_value(&harness, handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID,)
            .expect("settlement frame has structured live status")["pending"],
        false
    );
    argus.finish_require_no_indeterminate();

    drop(harness);
    let restarted = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let mut restarted_harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), restarted);
    restarted_harness.run_steps(2);
    let restarted_ctx = restarted_harness.ctx.clone();
    assert!(
        restarted_harness
            .state_mut()
            .dispatch_palette_action_for_test_with_ctx(&restarted_ctx, "view.code-editor"),
        "restart proof opens a real code editor through the product command route"
    );
    restarted_harness.run_steps(4);
    focus_code_text_surface(&restarted_harness);
    restarted_harness.run_steps(1);
    let restarted_code_pane = restarted_harness
        .state()
        .active_pane()
        .cloned()
        .expect("fresh mounted code editor owns the active pane");
    let restarted_indicator_author_id = undo_count_author_id(restarted_code_pane.as_ref());
    assert_eq!(
        mt035_bus_counts(&restarted_harness.ctx, &restarted_code_pane),
        (0, 0, false),
        "restart recovery model: undo history and interrupted in-memory compensation state are empty"
    );
    assert!(
        InteractionBus::with_try_lock(
            &InteractionBus::get_or_init(&restarted_harness.ctx),
            |bus| bus.undo_scope().is_empty(),
        )
        .expect("inspect the complete restarted undo scope"),
        "fresh mounted restart has no local/cross undo or redo history"
    );
    let mut restart_argus =
        CanonicalArgusDriver::bind(restarted_harness.state(), "mt035-v4-mounted-restart-empty");
    let restart_tree = restart_argus.inspect(&mut restarted_harness);
    assert_eq!(
        mt035_node_value(&restart_tree, &restarted_indicator_author_id).as_deref(),
        Some("Undo (0)"),
        "fresh mounted pane exposes its own exact empty undo count"
    );
    let restart_state = mt035_node_json_value(
        &restart_tree,
        handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID,
    )
    .expect("fresh restart state is persistently observable");
    let restart_pending_observers = mt035_pending_observer_author_ids(&restart_tree);
    assert!(
        restart_pending_observers.is_empty(),
        "strict restart cannot retain any pending click-completion observer: {restart_pending_observers:?}"
    );
    assert_eq!(restart_state["pending"], false);
    assert_eq!(restart_state["focused_local_count"], 0);
    assert_eq!(restart_state["cross_undo_count"], 0);
    assert_eq!(restart_state["transition"], serde_json::Value::Null);
    assert!(mt035_node_by_author_id(&restart_tree, CODE_EDITOR_TEXT_AUTHOR_ID).is_some());
    assert!(mt035_node_by_author_id(&restart_tree, "menu-edit").is_some());
    let restart_frame =
        mt035_capture_frame(&mut restarted_harness, &proof_dir, "06-restart-empty.png");
    assert_eq!(
        shell_indicator_value(&restarted_harness, &restarted_indicator_author_id).as_deref(),
        Some("Undo (0)"),
        "restart frame is captured from the fresh mounted pane's live empty state"
    );
    assert_eq!(
        mt035_live_json_value(
            &restarted_harness,
            handshake_native::app::MT035_UNDO_STATE_AUTHOR_ID,
        )
        .expect("restart frame has structured live status")["pending"],
        false
    );
    restart_argus.finish_require_no_indeterminate();

    let correlation_ids = [
        menu_open.correlation_id.as_str(),
        cross_undo.correlation_id.as_str(),
        retry_menu.correlation_id.as_str(),
        blocked.correlation_id.as_str(),
        local_menu.correlation_id.as_str(),
        local.correlation_id.as_str(),
    ];
    assert!(
        correlation_ids.iter().all(|value| !value.trim().is_empty()),
        "every canonical Argus action must retain a non-empty correlation identity"
    );
    assert_eq!(
        correlation_ids
            .iter()
            .copied()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        correlation_ids.len(),
        "each canonical Argus mutation must have its own correlation identity"
    );
    let (source_candidate_after_id, source_candidate_after) = mt035_source_candidate_identity();
    assert_eq!(
        source_candidate_after_id, source_candidate_id,
        "all MT-035 V4 trees and frames must come from one unchanged source candidate"
    );

    let actions = vec![
        mt035_action_proof(
            "menu-edit",
            &menu_open,
            "edit-menu-freshly-open-with-enabled-undo-leaves",
        ),
        mt035_action_proof(
            "menu.edit.undo-cross-pane",
            &cross_undo,
            "cross-pane-undo-is-exactly-pending-and-local-depth-is-unchanged",
        ),
        mt035_action_proof(
            "menu-edit",
            &retry_menu,
            "edit-menu-reopened-for-blocked-retry",
        ),
        mt035_action_proof(
            "menu.edit.undo-cross-pane",
            &blocked,
            "blocked-retry-is-typed-and-keeps-the-same-pending-operation",
        ),
        mt035_action_proof(
            "menu-edit",
            &local_menu,
            "edit-menu-reopened-for-independent-local-undo",
        ),
        mt035_action_proof(
            "menu.edit.undo",
            &local,
            "focused-local-undo-decrements-only-local-depth-and-keeps-pending-identity",
        ),
    ];
    let proof = serde_json::json!({
        "schema_id": "hsk.mt035-canonical-argus-proof@2",
        "work_packet": "WP-KERNEL-012",
        "microtask": "MT-035",
        "validation_round": "v4",
        "run_id": run_id,
        "source_candidate_id": source_candidate_id,
        "source_candidate": source_candidate,
        "source_candidate_after": source_candidate_after,
        "compensation_action_id": COMPENSATION_ACTION_ID,
        "focused_pane_id": code_pane.as_ref(),
        "restarted_focused_pane_id": restarted_code_pane.as_ref(),
        "actions": actions,
        "checkpoints": {
            "initial": {
                "tree": initial_tree,
                "tree_sha256": mt035_sha256_json(&initial_tree),
                "counts": {"local": 1, "cross": 1, "pending": false},
                "indicator_author_id": indicator_author_id,
            },
            "settlement": {
                "tree": settlement_tree,
                "tree_sha256": mt035_sha256_json(&settlement_tree),
                "status": settlement_state,
                "compensation_future_dropped_before_fifo_drain": true,
                "active_compensation_futures_before_fifo_drain": 0,
                "pending_after": false,
                "pending_click_completion_observers": settlement_pending_observers,
            },
            "restart": {
                "tree": restart_tree,
                "tree_sha256": mt035_sha256_json(&restart_tree),
                "status": restart_state,
                "counts": {"local": 0, "cross": 0, "pending": false},
                "indicator_author_id": restarted_indicator_author_id,
                "pending_click_completion_observers": restart_pending_observers,
            }
        },
        "frames": [
            {"phase": "before", "capture": initial_frame, "tree_sha256": mt035_sha256_json(&initial_tree)},
            {"phase": "edit-open", "capture": menu_open_frame, "tree_sha256": mt035_sha256_json(&menu_open.after)},
            {"phase": "cross-pane-pending", "capture": cross_pending_frame, "tree_sha256": mt035_sha256_json(&cross_undo.after)},
            {"phase": "local-undone-cross-pending", "capture": local_pending_frame, "tree_sha256": mt035_sha256_json(&local.after)},
            {"phase": "compensation-settled", "capture": settlement_frame, "tree_sha256": mt035_sha256_json(&settlement_tree)},
            {"phase": "restart-empty", "capture": restart_frame, "tree_sha256": mt035_sha256_json(&restart_tree)},
        ],
        "strict_finish": {
            "indeterminate_actions": 0,
            "unresolved_actions": 0,
            "pending_compensations": 0,
            "compensation_futures_in_flight": 0,
            "pending_click_completion_observers": 0,
        }
    });
    let proof_path = proof_dir.join("mt035-v4-canonical-argus-proof.json");
    std::fs::write(
        &proof_path,
        serde_json::to_vec_pretty(&proof).expect("serialize MT-035 V4 proof"),
    )
    .expect("write MT-035 V4 proof artifact after strict finish");
    assert_no_local_artifact_dir();
}

#[test]
fn canvas_compensation_failure_is_attributed_to_origin_board_across_a_b_a() {
    use handshake_native::undo_stack::AsyncUndoDirection;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("parked runtime");
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    {
        let registry = app.pane_registry();
        registry.lock().unwrap().insert(PaneRecord::new(
            PaneId::from("pane-a"),
            PaneType::AtelierEditor,
            "ws-a",
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
        let bar = app
            .tab_bar_states_mut()
            .get_mut(&PaneId::from("pane-a"))
            .expect("pane-a tab bar");
        bar.tabs = vec![handshake_native::tab_bar::TabState::new(
            PaneType::AtelierEditor,
        )];
        bar.active_index = 0;
    }
    app.begin_canvas_request_for_test("ws-a", "canvas-a");
    let board = app.mounted_canvas_board();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);

    let bus = InteractionBus::get_or_init(&harness.ctx);
    InteractionBus::with_try_lock(&bus, |bus| {
        bus.set_undo_runtime(runtime.handle().clone());
        let pending: handshake_native::undo_stack::UndoAsyncFn = Arc::new(|| {
            Box::pin(async {
                std::future::pending::<()>().await;
                UndoResult::ok()
            })
        });
        bus.push_undo_cross_pane(UndoAction::async_compensating(
            "canvas-a-action",
            "canvas A action",
            Arc::new(UndoResult::ok),
            Arc::new(UndoResult::ok),
            Arc::clone(&pending),
            pending,
        ));
        assert!(bus.undo_cross_pane().expect("dispatch A undo").ok);
    })
    .expect("bus lock");
    harness.state().deliver_canvas_compensation_for_test(
        "canvas-a-action",
        AsyncUndoDirection::Undo,
        "ws-a",
        "canvas-a",
        Err("A compensation failed".to_owned()),
    );

    harness
        .state_mut()
        .begin_canvas_request_for_test("ws-b", "canvas-b");
    harness.step();
    assert!(
        board.lock().unwrap().error.as_deref() != Some("A compensation failed"),
        "a late Canvas A failure must not paint the currently mounted Canvas B"
    );

    harness
        .state_mut()
        .begin_canvas_request_for_test("ws-a", "canvas-a");
    harness.step();
    assert_eq!(
        board.lock().unwrap().error.as_deref(),
        Some("A compensation failed"),
        "returning to Canvas A restores its attributable compensation failure"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-3 — POLICY-3 session-scoped: fresh scope empty + the type must NOT implement Serialize.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn fresh_scope_is_empty_and_session_scoped() {
    // A fresh scope (the only state that exists on app restart) holds nothing.
    let scope = UnifiedUndoScope::new();
    assert!(
        scope.is_empty(),
        "AC-3: a fresh scope is empty (session-scoped, never reloaded)"
    );
    // A fresh bus exposes an empty scope too (the bus lives in egui app data which is not persisted).
    let bus = InteractionBus::new();
    assert!(
        bus.undo_scope().is_empty(),
        "AC-3: a fresh bus's undo scope is empty"
    );
    assert_eq!(bus.local_undo_count(&pane("any")), 0);
}

/// AC-3 (the no-Serialize half): the undo scope + action + rings MUST NOT derive or implement
/// Serialize/Deserialize — a `#[derive(Serialize)]` would let the history be accidentally persisted,
/// which the session-scoped policy forbids. A source-level guard asserts neither the derive nor a serde
/// import is present in `src/undo_stack.rs`. (A compile-time guard via a `fn assert_not_serialize<T:
/// !Serialize>()` is not expressible on stable Rust, so the source guard is the field-correct proof.)
#[test]
fn undo_scope_does_not_implement_serialize() {
    let src = std::fs::read_to_string("src/undo_stack.rs").expect("read src/undo_stack.rs");
    // Scan only CODE lines (skip `//`/`///` doc comments — the module DOCUMENTS the no-Serialize policy
    // in prose, which must be allowed; what is forbidden is an actual derive / impl / serde import).
    let code: String = src
        .lines()
        .filter(|l| {
            let t = l.trim_start();
            !t.starts_with("//") && !t.starts_with("///")
        })
        .collect::<Vec<_>>()
        .join("\n");
    // No serde derive macro and no manual Serialize/Deserialize impl anywhere in the undo-scope code.
    for forbidden in [
        "derive(Serialize",
        "Serialize)",
        "Serialize,",
        "derive(Deserialize",
        "Deserialize)",
        "impl Serialize",
        "impl Deserialize",
        "use serde",
        "serde::",
    ] {
        assert!(
            !code.contains(forbidden),
            "AC-3 / POLICY-3: src/undo_stack.rs code must NOT contain {forbidden:?} — the undo scope is \
             session-scoped and must never be persisted; a serde derive/impl here is a contract FAILURE"
        );
    }
    // And the module documents the policy explicitly (impl-note requirement).
    assert!(
        src.contains("POLICY-3") && src.contains("session-scoped"),
        "POLICY-3 must be documented in the module"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-5 — POLICY-5 caps: 201 -> 200 (pane ring), 51 -> 50 (cross-pane ring).
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn pane_ring_caps_at_200_after_201_pushes() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut ring = PaneUndoRing::new(pane("p")); // default cap = PANE_RING_CAP (200)
    assert_eq!(PANE_RING_CAP, 200);
    for _ in 0..201 {
        ring.push(sync_action("z", log.clone()));
    }
    assert_eq!(
        ring.undo_len(),
        200,
        "AC-5: a cap-200 pane ring holds 200 after 201 pushes (oldest dropped)"
    );
}

#[test]
fn cross_pane_ring_caps_at_50_after_51_pushes() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut scope = UnifiedUndoScope::new();
    assert_eq!(CROSS_PANE_RING_CAP, 50);
    for _ in 0..51 {
        scope.push_cross_pane(sync_action("c", log.clone()));
    }
    assert_eq!(
        scope.cross_pane_undo_count(),
        50,
        "AC-5: the cap-50 cross-pane ring holds 50 after 51 pushes (oldest dropped)"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-6 — undo-count indicator helper + live shell header mount.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// A kittest harness rendering the undo-count indicator helper for a pane whose local ring depth the test
/// drives, then asserting the AccessKit `undo-count-{pane_id}` Label value tracks the count. The separate
/// `live_shell_header_undo_count_tracks_shared_bus_depth` test proves the mounted pane-header call site.
struct IndicatorApp {
    bus: Arc<Mutex<InteractionBus>>,
    pane_id: PaneId,
}

impl IndicatorApp {
    fn ui(&mut self, ctx: &egui::Context) {
        let theme = handshake_native::theme::HsTheme::Dark;
        let palette = theme.palette();
        egui::CentralPanel::default().show(ctx, |ui| {
            let count = self.bus.lock().unwrap().local_undo_count(&self.pane_id);
            render_undo_count_indicator(ui, &self.pane_id, count, &palette);
        });
    }
}

fn indicator_value(harness: &Harness<'_, IndicatorApp>, author_id: &str) -> Option<String> {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.value();
        }
    }
    None
}

#[test]
fn undo_count_indicator_tracks_ring_depth() {
    let pane_id = pane("pane-code");
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    let log = Arc::new(Mutex::new(Vec::new()));
    // Push 3 local actions.
    {
        let mut b = bus.lock().unwrap();
        for tag in ["a", "b", "c"] {
            b.push_undo_local(pane_id.clone(), sync_action(tag, log.clone()));
        }
    }
    let author_id = undo_count_author_id("pane-code");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(320.0, 80.0))
        .build_state(
            |ctx, a: &mut IndicatorApp| a.ui(ctx),
            IndicatorApp {
                bus: bus.clone(),
                pane_id: pane_id.clone(),
            },
        );
    harness.run();
    assert_eq!(
        indicator_value(&harness, &author_id).as_deref(),
        Some("Undo (3)"),
        "AC-6: the indicator shows the count after 3 pushes"
    );

    // Undo once -> count drops to 2.
    bus.lock().unwrap().undo(&pane_id);
    harness.run();
    assert_eq!(
        indicator_value(&harness, &author_id).as_deref(),
        Some("Undo (2)"),
        "AC-6: the indicator drops to 2 after one undo"
    );

    // HBR-VIS screenshot (best-effort on a GPU host); artifacts ONLY to the external root.
    match harness.render() {
        Ok(image) => {
            let dir =
                Path::new("../../../../Handshake_Artifacts/handshake-test/wp-kernel-012-mt-035");
            let _ = std::fs::create_dir_all(dir);
            let path = dir.join("MT-035-undo-count-indicator.png");
            let saved = image.save(&path).is_ok();
            println!(
                "AC-6 indicator screenshot: {}x{}, saved={saved} ({})",
                image.width(),
                image.height(),
                path.display()
            );
        }
        Err(e) => println!(
            "BLOCKER(non-fatal): MT-035 indicator screenshot unavailable (no wgpu adapter): {e}. \
             The AccessKit value proof (Undo (3) -> Undo (2)) stands as the AC-6 evidence."
        ),
    }
    assert_no_local_artifact_dir();
}

#[test]
fn live_shell_header_undo_count_tracks_shared_bus_depth() {
    let (app, _rt) = mt035_editor_shell();
    let pane_id = pane("pane-a");
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |b| {
        b.push_undo_local(pane_id.clone(), sync_action("header-count", log.clone()));
    })
    .expect("bus lock");
    harness.run_steps(1);

    let author_id = undo_count_author_id("pane-a");
    assert_eq!(
        shell_indicator_value(&harness, &author_id).as_deref(),
        Some("Undo (1)"),
        "AC-6 LIVE: the pane header emits the shared-bus undo count for pane-a"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-4 — POLICY-4 canvas compensating undo. The request-SHAPE binding is proven here without a live
// backend; the integration-feature proof below runs the round-trip against real PostgreSQL.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// The compensating-undo request shape: a canvas placement undo must DELETE the created placement via the
/// verified MT-026 route `/workspaces/:ws/loom/canvas-placements/:placement_id` — NOT the contract's
/// stale `PUT /canvas/{id}/graph`. This proves the binding (route + method) the cross-pane canvas undo
/// issues, using the same `CanvasBoardClient` request builder, WITHOUT a live backend.
#[test]
fn canvas_compensating_undo_uses_verified_delete_route() {
    use handshake_native::backend_client::{CanvasBoardClient, HttpMethod};
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let client = CanvasBoardClient::new("http://127.0.0.1:0", rt.handle().clone());
    // The undo = remove_placement_request (the compensating call the async undo_fn sends).
    let spec = client.remove_placement_request("ws-1", "placement-42");
    assert_eq!(
        spec.method,
        HttpMethod::Delete,
        "POLICY-4: canvas undo is a DELETE (compensating)"
    );
    assert!(
        spec.url
            .ends_with("/workspaces/ws-1/loom/canvas-placements/placement-42"),
        "POLICY-4: the compensating route is the verified MT-026 canvas-placements route, not PUT \
         /canvas/{{id}}/graph; got {}",
        spec.url
    );
    // The redo = re-place the SAME block at the SAME geometry (POST .../placements).
    let redo = client.place_block_request("ws-1", "canvas-9", "blk-7", 10.0, 20.0, 200.0, 120.0);
    assert_eq!(redo.method, HttpMethod::Post);
    assert!(redo
        .url
        .ends_with("/workspaces/ws-1/loom/canvas-boards/canvas-9/placements"));
}

/// AC-4 full round-trip: self-seed an owned workspace, two canonical blocks, a Canvas, and two
/// placements against Handshake-managed PostgreSQL. Drive the real shell Ctrl+Shift+Z consumer, then
/// prove the exact owned placement is absent after a fresh board reload while the unrelated placement
/// remains. Run with `cargo test --features integration --test test_undo_scope -- --test-threads=1`.
#[test]
#[cfg(feature = "integration")]
fn canvas_placement_undo_round_trip_live_pg() {
    use handshake_native::backend_client::{CanvasBoardClient, CreatedCanvasPlacement};

    let mut managed_backend = pg_proof_support::require_reachable_backend();
    let backend_binding = managed_backend.owned_backend_binding_receipt();
    println!("MT-035 owned backend binding: {backend_binding}");
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("MT-035 integration runtime");
    let base = managed_backend.base.clone();
    let http = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(2))
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("bounded MT-035 integration client");
    let suffix = format!(
        "{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );

    let (workspace_id, canvas_id, owned, unrelated, mut cleanup) = runtime.block_on(async {
        assert!(http
            .get(format!("{base}/health"))
            .send()
            .await
            .expect("live handshake_core health")
            .status()
            .is_success());
        let workspace: serde_json::Value =
            mt035_workspace_headers(http.post(format!("{base}/workspaces")))
                .json(&serde_json::json!({"name": format!("MT-035-{suffix}")}))
                .send()
                .await
                .expect("create MT-035 workspace")
                .json()
                .await
                .expect("workspace JSON");
        let workspace_id = workspace["id"].as_str().expect("workspace id").to_owned();
        let cleanup = Mt035WorkspaceCleanup {
            base: base.clone(),
            workspace_id: workspace_id.clone(),
            armed: true,
        };

        let import_block = |name: String, bytes_b64: &'static str| {
            mt035_proof_headers(http.post(format!("{base}/workspaces/{workspace_id}/loom/import")))
                .json(&serde_json::json!({
                    "bytes_b64": bytes_b64,
                    "original_filename": name,
                    "mime": "text/plain"
                }))
        };
        let first: serde_json::Value =
            import_block(format!("mt035-owned-{suffix}.txt"), "bXQwMzUtb3duZWQ=")
                .send()
                .await
                .expect("import owned block")
                .json()
                .await
                .expect("owned import JSON");
        let second: serde_json::Value = import_block(
            format!("mt035-unrelated-{suffix}.txt"),
            "bXQwMzUtdW5yZWxhdGVk",
        )
        .send()
        .await
        .expect("import unrelated block")
        .json()
        .await
        .expect("unrelated import JSON");
        let first_block = first["block_id"].as_str().expect("owned block id");
        let second_block = second["block_id"].as_str().expect("unrelated block id");

        let canvas: serde_json::Value = http
            .post(format!(
                "{base}/workspaces/{workspace_id}/loom/canvas-boards"
            ))
            .json(&serde_json::json!({"title": format!("MT-035 Canvas {suffix}")}))
            .send()
            .await
            .expect("create MT-035 canvas")
            .json()
            .await
            .expect("canvas JSON");
        let canvas_id = canvas["block_id"].as_str().expect("canvas id").to_owned();
        let client = CanvasBoardClient::new(&base, runtime.handle().clone());
        let owned = mt035_dispatch_created_placement(
            &client,
            client.place_block_request(
                &workspace_id,
                &canvas_id,
                first_block,
                40.0,
                40.0,
                200.0,
                120.0,
            ),
        )
        .await;
        let unrelated = mt035_dispatch_created_placement(
            &client,
            client.place_block_request(
                &workspace_id,
                &canvas_id,
                second_block,
                300.0,
                40.0,
                200.0,
                120.0,
            ),
        )
        .await;
        (workspace_id, canvas_id, owned, unrelated, cleanup)
    });

    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    app.set_runtime_handle(runtime.handle().clone());
    app.set_backend_base_url_for_test(&base, runtime.handle().clone());
    {
        let board = app.mounted_canvas_board();
        let mut board = board.lock().unwrap();
        board.workspace_id = workspace_id.clone();
        board.canvas_block_id = canvas_id.clone();
    }
    app.deliver_canvas_created_placement_for_test(
        &workspace_id,
        &canvas_id,
        CreatedCanvasPlacement {
            created_by_request: true,
            ..owned.clone()
        },
        "canvas: place owned MT-035 block",
        false,
    )
    .expect("register owned placement through production host drain");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let mut modifiers = egui::Modifiers::COMMAND;
    modifiers.ctrl = true;
    modifiers.shift = true;
    harness.key_press_modifiers(modifiers, egui::Key::Z);

    let mut fresh_board = serde_json::Value::Null;
    for _ in 0..100 {
        harness.step();
        fresh_board = runtime.block_on(async {
            http.get(format!(
                "{base}/workspaces/{workspace_id}/loom/canvas-boards/{canvas_id}"
            ))
            .send()
            .await
            .expect("fresh board after Ctrl+Shift+Z")
            .json()
            .await
            .expect("fresh board JSON")
        });
        let rows = fresh_board["placements"].as_array().expect("placements");
        if rows
            .iter()
            .all(|row| row["placement_id"].as_str() != Some(owned.placement_id.as_str()))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let rows = fresh_board["placements"].as_array().expect("placements");
    assert!(
        rows.iter()
            .all(|row| row["placement_id"].as_str() != Some(owned.placement_id.as_str())),
        "Ctrl+Shift+Z compensating DELETE removes the exact owned placement: {fresh_board}"
    );
    assert!(
        rows.iter()
            .any(|row| { row["placement_id"].as_str() == Some(unrelated.placement_id.as_str()) }),
        "the unrelated canonical placement survives the exact compensation: {fresh_board}"
    );

    drop(harness);
    runtime.block_on(cleanup.cleanup(&http));
    let owned_backend_pid = backend_binding["backend_pid"]
        .as_u64()
        .expect("owned backend binding carries exact PID");
    managed_backend.assert_cleanup();
    println!("MT-035 owned backend PID {owned_backend_pid} stopped and reaped");
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// FIX A (data-safety) — undo-history corruption: a `type -> line-op -> type` burst inside the 500ms
// coalesce window must KEEP the non-typing (line-op) entry. Before the fix the code-pane typing
// coalescer's `last_edit_at` was never reset when a NON-typing entry (format / line-op / cut / paste)
// was pushed, so the 2nd type's `replace_tail=true` clobbered the line-op entry via
// `replace_undo_local_tail`, silently dropping it from history. The fix: every non-typing code-edit
// entry routes through `interop_adapter::push_code_edit_undo`, which now calls
// `CodeEditorPanel::reset_text_edit_undo_batch_timing()` so the NEXT keystroke starts a FRESH entry
// instead of `replace_tail`-ing over the non-typing tail.
//
// Driven END-TO-END through the mounted shell + real bus — NO manual `push_code_edit_undo` seeding; the
// LIVE producer stages every entry, the pane factory drains each through the real bus boundary. The
// non-typing (line-op) step is driven DETERMINISTICALLY via `CodeEditorPanel::dispatch_action` (the EXACT
// arm the Ctrl+/ keybind resolves to — see `keymap.rs` `A::ToggleComment`), NOT a raw `Ctrl+/` chord
// through `Harness`: egui_kittest did not reliably deliver a modified `Slash` key event to the panel's
// keymap, but the undo-corruption defect lives at the coalescer/undo-stack boundary, not in key
// delivery — so we exercise the real production push+reset path without the flaky keymap layer.
//
// GUARD (regression teeth): if `push_code_edit_undo`'s `reset_text_edit_undo_batch_timing()` call is
// removed, the 2nd type coalesces (`replace_tail`) over the line-op entry E2 -> the ring holds only TWO
// entries and undo #2 reverts the typing burst instead of the line op -> this test FAILS. Verified by
// deleting the reset and observing `depth == 2`.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn undo_scope_type_around_line_op_keeps_all_three_entries_live() {
    let (app, _rt) = mt035_editor_shell();
    let code_panel = app.mounted_code_panel();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    let pane_id = PaneId::from("pane-a");
    // A multi-line Rust doc (the mounted code pane is ext "rs" -> the line-comment token is "//", so
    // Toggle Comment mutates the buffer). `s0` is the original document.
    let s0 = "alpha\nbeta\ngamma\n";
    code_panel.set_text(s0);
    // Caret at the start of line 1 ("beta"): "alpha\n" is 6 bytes, so byte offset 6 is that line start.
    code_panel.set_single_cursor(6);
    focus_code_text_surface(&harness);
    harness.run_steps(1);

    // (1) TYPE — the live producer stages + drains a FRESH typing undo entry E1 (no manual seeding).
    harness.event(egui::Event::Text("X".to_owned()));
    harness.run_steps(2);
    let s1 = code_panel.buffer().to_string();
    assert_ne!(
        s1, s0,
        "the first typed char mutated the mounted code buffer"
    );

    // (2) TOGGLE COMMENT — a NON-typing line-op entry E2. Driven DETERMINISTICALLY through the panel's
    // real action handler `CodeEditorPanel::dispatch_action(CodeEditorAction::ToggleComment)` — the EXACT
    // arm the Ctrl+/ keybind resolves to (`keymap.rs` binds `Ctrl+Slash -> A::ToggleComment`; `app.rs`
    // routes that action into `dispatch_action`). This stages `pending_line_op_undo`, and the factory
    // render on the next `run_steps` drains it through `interop_adapter::push_code_edit_undo` — the SINGLE
    // boundary FIX A resets the typing coalescer at — so the reset path is exercised for real. We call the
    // action directly (not a raw `Ctrl+/` chord) only because egui_kittest did not reliably deliver the
    // modified `Slash` key event to the panel keymap; the undo-corruption defect is unaffected by key
    // delivery, so this loses no coverage of the production push+reset boundary.
    code_panel.dispatch_action(CodeEditorAction::ToggleComment);
    harness.run_steps(2);
    let s2 = code_panel.buffer().to_string();
    assert_ne!(
        s2, s1,
        "ToggleComment commented the caret line (the non-typing undo entry E2); got {s2:?}"
    );

    // (3) TYPE again, still WITHIN the 500ms window of the first type (test frames are microseconds
    // apart). Before the fix this 2nd type coalesced (`replace_tail`) over E2 and DROPPED the line-op
    // entry; with the fix E2's push reset the coalescer, so this is a fresh entry E3.
    harness.event(egui::Event::Text("Y".to_owned()));
    harness.run_steps(2);
    let s3 = code_panel.buffer().to_string();
    assert_ne!(
        s3, s2,
        "the second typed char mutated the mounted code buffer"
    );

    // PROOF (FIX A): all THREE entries survive — the line-op entry was NOT clobbered by the coalescer.
    let depth = InteractionBus::with_try_lock(&InteractionBus::get_or_init(&harness.ctx), |b| {
        b.local_undo_count(&pane_id)
    })
    .expect("bus lock");
    assert_eq!(
        depth, 3,
        "FIX A: type -> line-op -> type records THREE undo entries; the non-typing line-op entry is not \
         silently dropped by the typing coalescer (got {depth})"
    );

    // And undoing reverts in correct reverse order s3 -> s2 -> s1 -> s0 (the line-op step is recovered).
    code_panel.request_undo_for_test();
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        s2,
        "FIX A: undo #1 reverts the 2nd typing edit (back to the commented line)"
    );
    code_panel.request_undo_for_test();
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        s1,
        "FIX A: undo #2 reverts the LINE-OP — the entry the old coalescer silently dropped from history"
    );
    code_panel.request_undo_for_test();
    harness.run_steps(2);
    assert_eq!(
        code_panel.buffer().to_string(),
        s0,
        "FIX A: undo #3 reverts the 1st typing edit, back to the original document"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// FIX B (perf) — a focused-IDLE rich pane pays NO per-frame doc serialization. Before the fix the widget
// serialized the WHOLE doc tree TWICE (doc_before + doc_after) on every focused frame, including
// caret-blink repaints with ZERO input events. The `doc_snapshot_count` seam advances ONLY on frames
// that carry input events; a real edit still records exactly one unified-undo entry.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

#[test]
fn undo_scope_rich_focused_idle_frames_take_no_doc_snapshot() {
    use handshake_native::rich_editor::document_model::node::BlockNode;
    use handshake_native::rich_editor::renderer::rich_editor_widget::{
        RichEditorState, RichEditorWidget,
    };

    let state = Arc::new(Mutex::new(RichEditorState::new(BlockNode::doc(vec![
        BlockNode::paragraph("Hello"),
    ]))));
    let rich_pane = pane("pane-rich-idle");
    state.lock().unwrap().undo_pane_id = Some(rich_pane.clone());

    let state_for_ui = Arc::clone(&state);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 300.0))
        .build_ui(move |ui| {
            handshake_native::app::HandshakeApp::install_fonts(ui.ctx());
            RichEditorWidget::new(Arc::clone(&state_for_ui)).show(ui);
        });
    harness.step();

    // Focus the editor surface, let focus settle, then run several focused-IDLE frames (no input
    // events). A focused rich editor blink-repaints every frame — these ARE the caret-blink repaints
    // FIX B targets, so `harness.step()` (single frame) is used, never `harness.run()` (would never
    // settle on the caret animation).
    focus_rich_surface(&harness);
    harness.step();
    let snapshots_before_idle = state.lock().unwrap().doc_snapshot_count();
    for _ in 0..8 {
        harness.step();
    }
    let snapshots_after_idle = state.lock().unwrap().doc_snapshot_count();
    assert_eq!(
        snapshots_after_idle, snapshots_before_idle,
        "FIX B: 8 focused-idle frames took {} doc snapshot(s); an empty-events frame must skip the O(n) \
         doc_before/doc_after serialization entirely",
        snapshots_after_idle - snapshots_before_idle
    );

    let bus = InteractionBus::get_or_init(&harness.ctx);
    let idle_undo =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        idle_undo, 0,
        "FIX B: focused-idle frames record NO undo entry (got {idle_undo})"
    );

    // A REAL edit still snapshots + records exactly one entry (undo behavior unchanged when events occur).
    harness.event(egui::Event::Text("Z".to_owned()));
    harness.step();
    let edited = state
        .lock()
        .unwrap()
        .block_plain_text(0)
        .unwrap_or_default();
    assert_ne!(
        edited, "Hello",
        "the typed char mutated the doc (got {edited:?})"
    );

    let snapshots_after_edit = state.lock().unwrap().doc_snapshot_count();
    assert_eq!(
        snapshots_after_edit,
        snapshots_after_idle + 1,
        "FIX B: exactly ONE snapshot pass on the single input-carrying frame (got {})",
        snapshots_after_edit - snapshots_after_idle
    );
    let edit_undo =
        InteractionBus::with_try_lock(&bus, |b| b.local_undo_count(&rich_pane)).expect("bus lock");
    assert_eq!(
        edit_undo, 1,
        "FIX B: a real edit still records exactly ONE unified-undo entry (got {edit_undo})"
    );
}
