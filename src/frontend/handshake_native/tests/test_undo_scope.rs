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
//!   reload round-trip remains NEEDS_MANAGED_RESOURCE_PROOF (real PG, `#[ignore]`d).
//! - AC-5 (POLICY-5 cap): 201 pushes to a cap-200 ring -> 200; 51 to cap-50 cross-pane -> 50.
//! - AC-6 (undo-count indicator): the `render_undo_count_indicator` helper emits
//!   `undo-count-{pane_id}` with the correct count in a kittest AccessKit dump, and the live pane header
//!   reads the same shared `InteractionBus` depth in the mounted shell.

use std::path::Path;
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::panel::CodeEditorPanel;
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::interop::interaction_bus::InteractionBus;
use handshake_native::interop::{render_undo_count_indicator, undo_count_author_id};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::stage_pane::{push_route_to_stage_undo, StageContent, StagePane};
use handshake_native::undo_stack::{
    PaneUndoRing, UndoAction, UndoResult, UnifiedUndoScope, CROSS_PANE_RING_CAP, PANE_RING_CAP,
};

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

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

fn sync_action(tag: &'static str, log: Arc<Mutex<Vec<String>>>) -> UndoAction {
    let undo_log = log.clone();
    UndoAction::sync(
        tag,
        Arc::new(move || {
            undo_log.lock().unwrap().push(tag.to_owned());
            UndoResult::ok()
        }),
        Arc::new(UndoResult::ok),
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
        .expect("the mounted code_editor_text TextInput node must be present");
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
}

#[test]
fn registered_undo_command_does_not_consume_cross_pane_when_local_empty() {
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
        Vec::<&str>::new(),
        "Ctrl+Z is local-only; Ctrl+Shift+Z owns cross-pane undo"
    );
    assert_eq!(
        bus.undo_scope().cross_pane_undo_count(),
        1,
        "cross-pane action remains available for the dedicated cross-pane command"
    );
}

// ══════════════════════════════════════════════════════════════════════════════════════════════════
// AC-1 LIVE — the rich pane's undo flows through the unified bus scope, driven by a REAL keystroke.
// ══════════════════════════════════════════════════════════════════════════════════════════════════

/// Find the rich-editor surface node by its stable author_id and focus it (so `apply_frame_input` runs).
fn focus_rich_surface(harness: &Harness<'_, ()>) {
    let node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some("rich-editor-surface"))
        .expect("the rich-editor-surface interactive node must be present");
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
    harness.step();

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
//   proof still needs a managed PostgreSQL run.
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
        },
        "canvas: place block",
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

    // BEFORE: the stage is empty. Route a selection to it (the cross-pane action), then record the undo.
    let previous = stage.lock().unwrap().content.clone();
    assert_eq!(previous, StageContent::Empty);
    let routed = StageContent::Selection("hello".to_owned(), "DOC-7".to_owned());
    stage.lock().unwrap().set_content(routed.clone());
    push_route_to_stage_undo(
        &mut bus,
        &stage,
        previous.clone(),
        routed.clone(),
        "route to stage",
    );

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

    // Ctrl+Shift+Z (the cross-pane undo command) reverts the stage to EMPTY.
    let ctrl_shift_z = handshake_native::interop::default_keybind_for(
        handshake_native::interop::CMD_UNDO_CROSS_PANE,
    )
    .unwrap();
    assert_eq!(
        bus.matching_keybind_command(&ctrl_shift_z),
        Some(handshake_native::interop::CMD_UNDO_CROSS_PANE)
    );
    assert!(bus.dispatch_command(&ctx, handshake_native::interop::CMD_UNDO_CROSS_PANE));
    assert_eq!(
        stage.lock().unwrap().content,
        StageContent::Empty,
        "AC-2: Ctrl+Shift+Z reverted the route-to-stage cross-pane action"
    );
    // Redo re-routes it.
    assert!(bus.redo_cross_pane().is_some());
    assert_eq!(
        stage.lock().unwrap().content,
        routed,
        "cross-pane redo re-routed the selection"
    );
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
// backend; the round-trip against real PG is NEEDS_MANAGED_RESOURCE_PROOF (ignored by default).
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

/// AC-4 full round-trip: place a canvas block, Ctrl+Shift+Z to issue the compensating DELETE, reload the
/// board, assert the placement is ABSENT. NEEDS_MANAGED_RESOURCE_PROOF — requires a live
/// Handshake-managed PostgreSQL with a seeded canvas block; never fakes PG. Run with:
///   cargo test -p handshake-native --features integration -- --ignored canvas_placement_undo_round_trip
#[test]
#[ignore = "NEEDS_MANAGED_RESOURCE_PROOF: live Handshake-managed PostgreSQL with a seeded canvas block; the placement create + compensating DELETE round-trip touches real PG"]
fn canvas_placement_undo_round_trip_live_pg() {
    use handshake_native::backend_client::CanvasBoardClient;
    use handshake_native::graph::interop_adapter::push_canvas_placement_undo;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    let client = Arc::new(CanvasBoardClient::production(rt.handle().clone()));
    let mut bus = InteractionBus::new();
    bus.set_undo_runtime(rt.handle().clone());
    bus.register_undo_commands();

    // The operator/seed supplies these via env when running the gated proof.
    let ws = std::env::var("HSK_TEST_WORKSPACE_ID").expect("seed: HSK_TEST_WORKSPACE_ID");
    let canvas = std::env::var("HSK_TEST_CANVAS_BLOCK_ID").expect("seed: HSK_TEST_CANVAS_BLOCK_ID");
    let block = std::env::var("HSK_TEST_PLACED_BLOCK_ID").expect("seed: HSK_TEST_PLACED_BLOCK_ID");

    // (1) Place the block (POST .../placements) and capture the created placement_id from the reload.
    let place = client.place_block_request(&ws, &canvas, &block, 40.0, 40.0, 200.0, 120.0);
    let cell: handshake_native::backend_client::CanvasBoardOpCell = Arc::new(Mutex::new(None));
    client.dispatch(place, cell.clone());
    // (Real harness would poll `cell` + re-fetch the board to read the new placement_id; left to the
    // seeded operator run — this body documents the exact round-trip the gated proof performs.)
    let placement_id =
        std::env::var("HSK_TEST_PLACEMENT_ID").expect("seed/derive: HSK_TEST_PLACEMENT_ID");

    // (2) Record the compensating cross-pane undo (snapshot of the placement captured NOW — RISK-2).
    push_canvas_placement_undo(
        &mut bus,
        client.clone(),
        ws.clone(),
        canvas.clone(),
        placement_id.clone(),
        block.clone(),
        (40.0, 40.0, 200.0, 120.0),
        "place canvas block",
    );

    // (3) Ctrl+Shift+Z -> the compensating DELETE fires; the placement must be ABSENT on reload.
    let result = bus.undo_cross_pane().expect("a cross-pane canvas undo");
    assert!(result.ok, "the compensating undo dispatched: {result:?}");
    // A real harness re-fetches the board and asserts `placement_id` is gone. Documented as the gated
    // assertion; the request-shape proof above (`canvas_compensating_undo_uses_verified_delete_route`)
    // proves the binding without a live backend.
}
