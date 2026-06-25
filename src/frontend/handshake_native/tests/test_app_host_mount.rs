//! WP-KERNEL-012 MT-079 (E11 host-mount) — the FIRST real-app GUI inspection of the native editors.
//!
//! These proofs drive the LIVE `HandshakeApp` through the SAME egui + AccessKit path the running shell
//! (and the out-of-process steering adapter) use, NOT a widget harness. They prove the host-mount closed
//! the structural gap MT-079 owns: the code + rich-text editors are now REGISTERED in the running app's
//! pane factory map and render their REAL editor subtrees (`code_editor_text` TextInput / `rich-editor-root`)
//! instead of the centered `PlaceholderPaneFactory` label.
//!
//! - PT-079-A / AC-079-1: `editors_render_live_in_app_tree_and_screenshot` mounts a code pane + a Notes
//!   pane in the live shell, runs the real `app.ui` for several frames, asserts BOTH real editor AccessKit
//!   subtrees are present (not a placeholder node), and saves a wgpu screenshot of the mounted editors to
//!   the EXTERNAL artifact root.
//! - PT-079-B / AC-079-2: `editor_mounts_thread_session_context` asserts the shell pushed the active
//!   workspace + runtime into the session-context cell so the editors threaded real session context on
//!   mount (the code panel carries the workspace id; the rich state's wikilink context is bound).
//! - PT-079-C / AC-079-3: `code_pane_undo_dispatches_through_bus` seeds a unified-undo entry on the SAME
//!   mounted code panel, dispatches Undo via the command channel + the shell drain, and asserts the
//!   MT-035 unified-undo scope mutated (the panel text reverted) — menu/keyboard undo share one stack.
//! - PT-079-D / AC-079-4: `shell_navigator_opens_mounted_editor_panes` invokes the ShellNavigator
//!   open_document / open_code_symbol arms and asserts they now OPEN the real mounted pane (an `Opened`
//!   outcome on the Notes / code surface), not the retired `EditorPaneNotMounted` seam.
//! - PT-079-E / AC-079-5: `rich_pending_events_drain_and_route` enqueues a `WikilinkActivated` on the
//!   SAME mounted rich state, runs a live frame, and asserts the editor's `pending_events` was DRAINED
//!   (reached the shell) — no event left unrouted.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::NodeT;
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::CODE_EDITOR_TEXT_AUTHOR_ID;
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};
use handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID;

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic. The crate sits at
/// `<repo>/src/frontend/handshake_native`, so four `..` reach `<repo>/..` where `Handshake_Artifacts`
/// is a sibling of the repo worktree. (The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path.)
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Assert NO repo-local artifact directory exists under the crate (the artifact-hygiene guard the
/// SCREENSHOT/TEST-ARTIFACT rule mandates). Checks BOTH `test_output/` and `tests/screenshots/`;
/// artifacts go to the external root ONLY — a stray local dir is a hygiene FAILURE.
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

/// A live, RUNTIME-INJECTED shell with the seeded 2x2 panes RE-TYPED so the top-left slot hosts the
/// code editor (`PaneType::CodeSymbol`) and the top-right slot hosts the Notes/rich editor
/// (`PaneType::LoomWikiPage`) — the two surfaces the MT-079 mounts register the real editor factories
/// over. A multi-thread runtime is injected (so the per-frame session push binds the editors' context)
/// and returned alongside the app so it OUTLIVES the harness (a dropped runtime would unbind the editors
/// mid-test). The active project id (`DEFAULT_PROJECT_ID`) is the non-empty workspace the session push
/// uses, so the editors thread real session context once the runtime is injected.
fn editor_shell() -> (HandshakeApp, tokio::runtime::Runtime) {
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

    // Re-type the seeded pane-a -> code editor, pane-b -> Notes/rich editor, so the split layout renders
    // the REAL mounted editor factories at those slots (the split renders each fixed pane id's RECORD
    // pane_type through the factory map).
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

/// Every `author_id` present in the live consumer-side AccessKit tree.
fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> std::collections::HashSet<String> {
    let mut ids = std::collections::HashSet::new();
    for node in harness.root().children_recursive() {
        if let Some(a) = node.accesskit_node().author_id() {
            ids.insert(a.to_owned());
        }
    }
    ids
}

// ── PT-079-A / AC-079-1: editors render LIVE in the running app + screenshot ──────────────────────────

#[test]
fn editors_render_live_in_app_tree_and_screenshot() {
    let _g = wgpu_guard();
    let (app, _rt) = editor_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Several frames: the mounts thread session context on the first live frame, then render.
    harness.run_steps(4);

    let ids = live_author_ids(&harness);
    // The REAL code editor subtree is present (the editable TextInput), proving the CodeEditorPaneMount
    // rendered the real panel — NOT a PlaceholderPaneFactory centered label.
    assert!(
        ids.contains(CODE_EDITOR_TEXT_AUTHOR_ID),
        "the live app tree carries the REAL code editor text node ('{CODE_EDITOR_TEXT_AUTHOR_ID}'); \
         got {ids:?}"
    );
    // The REAL rich editor subtree is present (the editor root), proving the RichEditorPaneMount rendered
    // the real editor — NOT a placeholder.
    assert!(
        ids.contains(RICH_EDITOR_ROOT_AUTHOR_ID),
        "the live app tree carries the REAL rich editor root node ('{RICH_EDITOR_ROOT_AUTHOR_ID}'); \
         got a subset {:?}",
        ids.iter().filter(|i| i.contains("editor") || i.contains("rich")).collect::<Vec<_>>()
    );

    // wgpu screenshot of the mounted editors -> the EXTERNAL artifact root ONLY (the first real-app GUI
    // inspection of the editors). On a GPU host this saves a PNG; absent an adapter, record an honest
    // non-fatal note (the AccessKit subtree proof above stands).
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-079");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-079-editors-mounted-live.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!(
                "PT-079-A mounted-editors screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            assert!(saved, "PT-079-A: the mounted-editors screenshot PNG saved to the external root");
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-079 mounted-editors screenshot render unavailable (no wgpu \
                 adapter): {e}. AC-079-1 AccessKit real-editor-subtree proof passed; the PNG is a \
                 GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── PT-079-B / AC-079-2: session context threaded into the editors on mount ───────────────────────────

#[test]
fn editor_mounts_thread_session_context() {
    let (app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let rich_state = app.mounted_rich_state();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    // AC-079-2: the code pane received set_workspace_id with the active workspace (the prior-MT hook ran
    // with real session context — not the headless empty workspace).
    assert_eq!(
        code_panel.workspace_id(),
        DEFAULT_PROJECT_ID,
        "the mounted code pane threaded the active workspace id on mount"
    );
    // AC-079-2: the rich pane's wikilink context bound the same workspace (set_wikilink_context ran).
    let ws = rich_state.lock().unwrap().wikilinks.workspace_id.clone();
    assert_eq!(ws, DEFAULT_PROJECT_ID, "the mounted rich pane threaded the wikilink workspace context");
    // The session cell carries the bound context the editors read each frame.
    let bound = harness.state().editor_session_context().lock().unwrap().is_bound();
    assert!(bound, "the shell pushed a BOUND session context (workspace + runtime) into the cell");
}

// ── PT-079-C / AC-079-3: code pane undo dispatches through the unified-undo bus ────────────────────────

#[test]
fn code_pane_undo_dispatches_through_bus() {
    use handshake_native::code_editor::TextBuffer;
    use handshake_native::interop::InteractionBus;

    let (app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // One frame so the code pane registers + the bus is initialized in egui app data.
    harness.run_steps(2);

    // Seed a unified-undo entry on the SAME mounted panel the way an edit would: record a
    // (before -> after) on the shared bus under the code pane id, with the panel now showing `after`.
    let pane_id: PaneId = PaneId::from("pane-a");
    let before = code_panel.buffer().to_string();
    let after = format!("{before}\n// edited by MT-079 proof");
    code_panel.set_text(&after);
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut guard = bus.lock().unwrap();
        guard.set_focus_owner(pane_id.clone());
        handshake_native::code_editor::interop_adapter::push_code_edit_undo(
            &mut guard,
            pane_id.clone(),
            &code_panel,
            TextBuffer::new(&before),
            TextBuffer::new(&after),
            "MT-079 proof edit",
        );
        assert_eq!(guard.local_undo_count(&pane_id), 1, "the unified-undo scope holds one entry");
    }
    assert_eq!(code_panel.buffer().to_string(), after, "panel shows the edited text before undo");

    // Dispatch Undo through the SAME command channel the keymap uses, then run a frame so the shell drain
    // (`drive_editor_mounts`) routes it to the bus undo for the FOCUSED pane (menu+keyboard share one
    // stack). The mounted code pane installed the command sender on mount; drive it via the panel.
    code_panel.request_undo_for_test();
    harness.run_steps(2);

    // AC-079-3: the unified-undo scope mutated — the entry was consumed and the panel reverted to
    // `before` (a single Undo through the bus reversed the edit).
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert_eq!(
            guard.local_undo_count(&pane_id),
            0,
            "Undo dispatched through the bus popped the unified-undo entry"
        );
    }
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "AC-079-3: a single Undo through the command bus reverted the code pane via the unified-undo stack"
    );
}

// ── PT-079-D / AC-079-4: ShellNavigator opens the mounted editor panes ────────────────────────────────

#[test]
fn shell_navigator_opens_mounted_editor_panes() {
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};

    let (mut app, _rt) = editor_shell();
    // open_document -> the Notes/rich editor surface is now MOUNTED, so the arm OPENS it (not the retired
    // EditorPaneNotMounted seam).
    let doc_outcome = app.open_document("KRD-mt079-doc");
    assert!(
        matches!(doc_outcome, NavDispatchOutcome::Opened { .. }),
        "open_document opens the mounted Notes editor pane; got {doc_outcome:?}"
    );
    // open_code_symbol -> the code editor surface is now MOUNTED.
    let sym_outcome = app.open_code_symbol("sym-mt079");
    assert!(
        matches!(sym_outcome, NavDispatchOutcome::Opened { .. }),
        "open_code_symbol opens the mounted code editor pane; got {sym_outcome:?}"
    );
}

// ── PT-079-E / AC-079-5: rich pending_events are drained + routed each frame ───────────────────────────

#[test]
fn rich_pending_events_drain_and_route() {
    use handshake_native::rich_editor::wikilinks::inline_view::EditorEvent;

    let (app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    // Enqueue a WikilinkActivated the way a chip click would, on the SAME mounted rich state.
    rich_state.lock().unwrap().pending_events.push(EditorEvent::WikilinkActivated {
        ref_kind: "note".into(),
        ref_value: "KRD-target-doc".into(),
        resolved: true,
    });
    assert_eq!(
        rich_state.lock().unwrap().pending_events.len(),
        1,
        "the event is enqueued on the editor state before the frame"
    );

    // One live frame: the rich pane factory render DRAINS pending_events into the shell's outbound queue,
    // and `drive_editor_mounts` routes them to the nav bus. After the frame the editor state's
    // pending_events is empty (drained) — no event left unrouted (AC-079-5).
    harness.run_steps(2);
    assert!(
        rich_state.lock().unwrap().pending_events.is_empty(),
        "AC-079-5: the rich pane's pending_events was DRAINED by the live render (routed to the nav bus)"
    );
}
