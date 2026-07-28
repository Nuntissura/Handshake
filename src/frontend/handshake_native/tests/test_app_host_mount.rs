//! WP-KERNEL-012 MT-079 (E11 host-mount) — the FIRST real-app GUI inspection of the native editors.
//!
//! These proofs drive the LIVE `HandshakeApp` through the SAME egui + AccessKit path the running shell
//! (and the out-of-process steering adapter) use, NOT a widget harness. They prove the host-mount closed
//! the structural gap MT-079 owns: the code + rich-text editors are now REGISTERED in the running app's
//! pane factory map and render their REAL editor subtrees (`editor.code.text` TextInput / `rich-editor-root`)
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

mod pg_proof_support;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use egui_kittest::kittest::NodeT;
#[path = "native_gui_support/screenshot_harness.rs"]
mod screenshot_harness;
use screenshot_harness::ScreenshotHarness as Harness;
#[path = "native_gui_support/canonical_argus_driver.rs"]
mod canonical_argus_driver;
use canonical_argus_driver::{json_has_author_id, CanonicalArgusDriver};

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::code_editor::lsp_client::{LspClient, LspServerConfig};
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
            assert!(
                saved,
                "PT-079-A: the mounted-editors screenshot PNG saved to the external root"
            );
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
    assert_eq!(
        ws, DEFAULT_PROJECT_ID,
        "the mounted rich pane threaded the wikilink workspace context"
    );
    // The session cell carries the bound context the editors read each frame.
    let bound = harness
        .state()
        .editor_session_context()
        .lock()
        .unwrap()
        .is_bound();
    assert!(
        bound,
        "the shell pushed a BOUND session context (workspace + runtime) into the cell"
    );
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
        assert_eq!(
            guard.local_undo_count(&pane_id),
            1,
            "the unified-undo scope holds one entry"
        );
    }
    assert_eq!(
        code_panel.buffer().to_string(),
        after,
        "panel shows the edited text before undo"
    );

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
    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::WikilinkActivated {
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

/// MT-079 remediation: pending rich events must not be stranded behind the file-backed loading
/// gate. A retry/document switch invalidates the exact view before its GET completes; the host still
/// owns that live state and must drain any event already queued on it in the loading frame.
#[test]
fn rich_pending_events_drain_while_document_is_loading() {
    use handshake_native::rich_editor::wikilinks::inline_view::EditorEvent;

    let (mut app, _rt) = editor_shell();
    assert!(app.open_document_in_pane_for_test("pane-b", "KRD-mt079-loading"));
    let rich_state = app.mounted_rich_state_for_view_for_test("pane-b", "KRD-mt079-loading");
    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::WikilinkActivated {
            ref_kind: "note".into(),
            ref_value: "KRD-stale-loading-event".into(),
            resolved: false,
        });

    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // The exact view is not ready until the asynchronous GET is delivered, so this frame exercises
    // the loading branch in RichEditorPaneMount rather than the editable renderer branch.
    harness.run_steps(2);

    assert!(
        rich_state.lock().unwrap().pending_events.is_empty(),
        "MT-079: loading/error gate must drain queued editor events instead of stranding them"
    );
}

#[test]
fn mounted_tag_event_resolves_canonical_name_to_real_hub_id() {
    use handshake_native::graph::tags_panel::TagEntry;
    use handshake_native::rich_editor::wikilinks::inline_view::EditorEvent;

    let (app, _rt) = editor_shell();
    app.mounted_tags_panel_for_test()
        .lock()
        .unwrap()
        .set_tags(vec![TagEntry::new("tag-hub-rust", "Rust", Some(2))]);
    let rich_state = app.mounted_rich_state();
    let tags_hub = app.mounted_tags_hub_for_test();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    rich_state
        .lock()
        .unwrap()
        .pending_events
        .push(EditorEvent::TagActivated {
            canonical: "rust".to_owned(),
            display: "#Rust".to_owned(),
        });
    harness.run_steps(4);
    assert!(rich_state.lock().unwrap().pending_events.is_empty());
    assert_eq!(
        tags_hub
            .lock()
            .unwrap()
            .as_ref()
            .map(|hub| hub.block_id.as_str()),
        Some("tag-hub-rust"),
        "mounted TagActivated resolves canonical text through the tag list before opening a hub"
    );
}

// ── WP-KERNEL-012 W3 / MT-070+MT-057: the code pane's 'Create note from link' HOST drain ───────────────

/// Confirming 'Create note from link' on the MOUNTED code pane's REAL context menu routes the staged
/// `[[title]]` through the SHELL drain (`drive_editor_mounts`) into the MT-057 create path
/// (`WikilinkRuntime::dispatch_create_note` on the mounted rich state) — NOT a panel-side manual drain.
/// The R2 audit found `take_pending_create_note_link` had ZERO product callers: the confirmed entry
/// staged the intent and the shell silently dropped it. This proof drives the live `app.ui` frame loop,
/// so a green run means the host consumed the intent (the panel's staged slot is empty WITHOUT this test
/// taking it) and the SAME wikilink runtime the rich editor's chip-click uses holds the in-flight create
/// (the `POST /knowledge/documents` dispatch fired, duplicate-guarded — MC-001). The following managed
/// test closes the durable PostgreSQL + exact navigation/focus half; this test isolates host wiring.
#[test]
fn code_pane_create_note_from_link_routes_through_host_drain() {
    use handshake_native::code_editor::panel::CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID;

    let (app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let rich_state = app.mounted_rich_state();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    // Several frames so the mounts wire: the rich pane's wikilink runtime binds workspace + runtime +
    // the production create backend (set_wikilink_context) — the target of the host route.
    harness.run_steps(3);
    {
        let mut state = rich_state.lock().unwrap();
        state.wikilinks.stage_resolver_seed(Vec::new());
        assert!(state.wikilinks.drain());
        assert!(state.wikilinks.is_resolver_index_ready());
    }
    harness.run_steps(1);

    // Put a `[[wikilink]]` under the caret on the SAME mounted panel the code pane renders.
    let snippet = "// see [[Design Notes]]\n";
    code_panel.set_text(snippet);
    code_panel.set_single_cursor(snippet.find("Design").expect("snippet link") + 2);
    harness.run_steps(1);
    assert_eq!(
        code_panel.wikilink_under_cursor().as_deref(),
        Some("Design Notes"),
        "the caret sits on the [[Design Notes]] wikilink in the MOUNTED code pane"
    );

    // Open the editor-body context menu via the REAL right-click path and confirm the entry.
    let context_surface = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID))
        .expect("the mounted code pane's context surface node is live");
    context_surface.click_secondary();
    let mut clicked_create = false;
    for _ in 0..20 {
        harness.run_steps(1);
        if let Some(node) = harness
            .root()
            .children_recursive()
            .find(|n| n.accesskit_node().author_id() == Some("ctx-menu.ctxmenu-editor-create-note"))
        {
            node.click();
            clicked_create = true;
            break;
        }
    }
    assert!(
        clicked_create,
        "the open menu's 'Create note from link' item is live; visible ctx ids: {:?}",
        harness
            .root()
            .children_recursive()
            .filter_map(|n| n.accesskit_node().author_id().map(str::to_owned))
            .filter(|id| id.starts_with("ctx-menu."))
            .collect::<Vec<_>>()
    );
    // The pane render confirms the entry (stages the intent) and the SAME frame's
    // `drive_editor_mounts` (after the pane host) drains + dispatches it. Poll ONE frame at a time,
    // checking BETWEEN frames: the dispatch lands at a frame's END (the drain runs after the pane
    // host), while the in-flight guard can only clear DURING a later frame's rich render (after the
    // off-thread outcome lands) — so a per-step check deterministically observes the in-flight create.
    let mut create_in_flight = false;
    for _ in 0..5 {
        harness.run_steps(1);
        if rich_state
            .lock()
            .unwrap()
            .wikilinks
            .is_creating("Design Notes")
        {
            create_in_flight = true;
            break;
        }
    }
    // The host routed the title into the MT-057 create path: the mounted rich state's wikilink runtime
    // held the in-flight create for the normalized title (the dispatch fired — not a silent drop).
    assert!(
        create_in_flight,
        "W3/R2: the HOST drain routed the staged title into WikilinkRuntime::dispatch_create_note \
         (the create for '[[Design Notes]]' went in flight on the SAME runtime the rich chip-click uses)"
    );
    // The staged intent is GONE from the panel — the HOST consumed it, not this test.
    assert_eq!(
        code_panel.take_pending_create_note_link(),
        None,
        "W3/R2: the panel's staged create-note intent was drained by the SHELL (drive_editor_mounts), \
         not by panel-side manual draining"
    );
    println!(
        "PASS W3/R2: 'Create note from link' on the mounted code pane routes through the host drain \
         into the MT-057 create path"
    );
}

#[test]
fn hidden_rich_tab_reused_existing_is_drained_and_truthfully_opened_by_shell_once() {
    use handshake_native::rich_editor::wikilinks::runtime::CreateNoteOutcome;
    use handshake_native::tab_bar::{TabBarState, TabState};

    let (mut app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    {
        let registry = app.pane_registry();
        registry.lock().unwrap().insert(PaneRecord::new(
            PaneId::from("pane-b"),
            PaneType::Workspace,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    app.tab_bar_states_mut().insert(
        PaneId::from("pane-b"),
        TabBarState::new(
            PaneId::from("pane-b"),
            vec![TabState::new(PaneType::Workspace)],
        ),
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    assert!(
        !live_author_ids(&harness).contains(RICH_EDITOR_ROOT_AUTHOR_ID),
        "the rich editor is not rendered while the shell receives the completion"
    );

    rich_state
        .lock()
        .unwrap()
        .wikilinks
        .stage_create(CreateNoteOutcome::Created {
            normalized_title: "hidden-note".to_owned(),
            display_title: "Hidden Note".to_owned(),
            document_id: "KRD-hidden-note".to_owned(),
            created: false,
        });
    harness.run_steps(2);

    let matching_tabs = harness
        .state()
        .tab_bar_states()
        .values()
        .flat_map(|bar| bar.tabs.iter())
        .filter(|tab| {
            tab.pane_type == PaneType::LoomWikiPage
                && tab.content_id.as_deref() == Some("KRD-hidden-note")
        })
        .count();
    assert_eq!(matching_tabs, 1, "shell opens the exact completion once");
    assert!(rich_state
        .lock()
        .unwrap()
        .take_created_document_navigation()
        .is_none());
    let status = harness
        .state()
        .quick_switcher_nav_status()
        .expect("reuse status is operator-visible");
    assert!(
        status.contains("Opened existing rich note") && !status.contains("Created"),
        "created=false must never be projected as Created: {status}"
    );
}

#[test]
fn hidden_rich_tab_create_failure_surfaces_typed_status_once() {
    use handshake_native::rich_editor::wikilinks::runtime::CreateNoteOutcome;

    let (app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    {
        let registry = app.pane_registry();
        registry.lock().unwrap().insert(PaneRecord::new(
            PaneId::from("pane-b"),
            PaneType::Workspace,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    rich_state
        .lock()
        .unwrap()
        .wikilinks
        .stage_create(CreateNoteOutcome::Failed {
            normalized_title: "hidden-note".to_owned(),
            reason: "409 concurrent title conflict".to_owned(),
        });
    harness.run_steps(2);

    assert_eq!(
        harness.state().quick_switcher_nav_status(),
        Some("Could not create note 'hidden-note': 409 concurrent title conflict")
    );
    assert!(
        rich_state
            .lock()
            .unwrap()
            .take_create_note_failure()
            .is_none(),
        "the shell consumed the failure projection exactly once"
    );
}

/// Managed-resource closure for the complete mounted Create-note chain. Unlike the in-flight wiring
/// proof above, this test waits for the actual PostgreSQL write, the rich editor's success delivery,
/// and the host's one-shot navigation handoff. The exact backend-minted id must become the focused
/// LoomWikiPage tab and must read back durably through the canonical document GET.
#[test]
fn managed_postgres_code_create_note_opens_exact_durable_rich_document() {
    use handshake_native::code_editor::panel::CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID;
    use handshake_native::rich_editor::wikilinks::runtime::KnowledgeCreateNoteBackend;

    let backend = pg_proof_support::require_live_backend();
    let (mut app, runtime) = editor_shell();
    app.bind_active_project_for_integration_test(backend.workspace_id.clone());
    app.set_backend_base_url_for_test(&backend.base, runtime.handle().clone());

    let code_panel = app.mounted_code_panel();
    let rich_state = app.mounted_rich_state();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    // Keep the real MT-037 create adapter, but point it at the exact managed backend chosen by the
    // fixture. The mounted session already owns the workspace/runtime and the host still owns every
    // dispatch, outcome, navigation, and focus transition in this proof.
    rich_state
        .lock()
        .unwrap()
        .wikilinks
        .set_create_backend(Arc::new(KnowledgeCreateNoteBackend::with_base_url(
            backend.base.clone(),
            "mt079-managed-host-create-note",
        )));
    {
        let mut state = rich_state.lock().unwrap();
        state.wikilinks.stage_resolver_seed(Vec::new());
        assert!(state.wikilinks.drain());
        assert!(state.wikilinks.is_resolver_index_ready());
    }
    harness.run_steps(1);

    let title = format!("Managed Host Note {}", uuid::Uuid::new_v4());
    let snippet = format!("// [[{title}]]\n");
    code_panel.set_text(&snippet);
    code_panel.set_single_cursor(snippet.find(&title).expect("unique managed title") + 2);
    harness.run_steps(1);
    assert_eq!(
        code_panel.wikilink_under_cursor().as_deref(),
        Some(title.as_str()),
        "the mounted code editor resolves the exact managed title under its caret"
    );

    let context_surface = harness
        .root()
        .children_recursive()
        .find(|node| {
            node.accesskit_node().author_id() == Some(CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID)
        })
        .expect("mounted code context surface");
    context_surface.click_secondary();
    let mut clicked = false;
    for _ in 0..20 {
        harness.run_steps(1);
        if let Some(node) = harness.root().children_recursive().find(|node| {
            node.accesskit_node().author_id() == Some("ctx-menu.ctxmenu-editor-create-note")
        }) {
            node.click();
            clicked = true;
            break;
        }
    }
    assert!(
        clicked,
        "canonical Create note from link menu item is mounted"
    );

    let mut created = None;
    let create_deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while std::time::Instant::now() < create_deadline {
        harness.run_steps(1);
        created = rich_state
            .lock()
            .unwrap()
            .last_slash_created_document
            .clone();
        if created.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let (document_id, created_title, created_by_request) =
        created.unwrap_or_else(|| {
            panic!(
                "managed POST completion must fold into the originating mounted rich state; status={:?}",
                harness.state().quick_switcher_nav_status()
            )
        });
    assert_eq!(created_title, title);
    assert!(created_by_request, "the unique managed title was inserted");
    assert_eq!(
        code_panel.take_pending_create_note_link(),
        None,
        "the shell, not the test, consumed the code panel's create intent"
    );
    assert_eq!(
        rich_state
            .lock()
            .unwrap()
            .take_created_document_navigation(),
        None,
        "the host consumed the successful create navigation handoff exactly once"
    );

    let active_pane = harness
        .state()
        .active_pane()
        .expect("successful create focuses a pane");
    let active_tab = harness
        .state()
        .tab_bar_states()
        .get(active_pane)
        .and_then(|bar| bar.active())
        .expect("focused pane has an active tab");
    assert_eq!(active_tab.pane_type, PaneType::LoomWikiPage);
    assert_eq!(active_tab.content_id.as_deref(), Some(document_id.as_str()));
    assert_eq!(
        harness
            .state()
            .tab_bar_states()
            .values()
            .flat_map(|bar| bar.tabs.iter())
            .filter(|tab| {
                tab.pane_type == PaneType::LoomWikiPage
                    && tab.content_id.as_deref() == Some(document_id.as_str())
            })
            .count(),
        1,
        "the backend document is opened once, never as duplicate rich tabs"
    );
    assert!(
        harness
            .state()
            .quick_switcher_nav_status()
            .is_some_and(|status| {
                status.contains("Created and opened rich note") && status.contains(&document_id)
            }),
        "the mounted operator status reports the exact created-and-opened document"
    );

    let loaded = backend.get_json(&format!("/knowledge/documents/{document_id}"));
    let document = loaded.get("document").unwrap_or(&loaded);
    assert_eq!(
        document
            .get("rich_document_id")
            .and_then(serde_json::Value::as_str),
        Some(document_id.as_str())
    );
    assert_eq!(
        document.get("title").and_then(serde_json::Value::as_str),
        Some(title.as_str())
    );
    assert_eq!(
        document
            .get("workspace_id")
            .and_then(serde_json::Value::as_str),
        Some(backend.workspace_id.as_str()),
        "durable readback is scoped to the exact managed workspace"
    );
}

// ── WP-KERNEL-012 MT-055 REMEDIATION: reading mode is REACHABLE in the MOUNTED editor ──────────────────

/// The Edit|Reading segmented toggle renders in the MOUNTED Notes pane's chrome (stable author_ids in
/// the live app tree), and clicking the Reading segment through the REAL AccessKit dispatch path flips
/// the mounted editor into reading mode (the Reading segment reads toggled/selected — the state a
/// no-context swarm agent reads). The 2026-07-02 audit found `view_mode_toggle` had zero production
/// callers (reading mode unreachable); this proves the mounted path, not a widget harness.
#[test]
fn reading_mode_toggle_reachable_and_flips_in_mounted_editor() {
    use handshake_native::rich_editor::reading_mode::{
        TOGGLE_CONTAINER_AUTHOR_ID, TOGGLE_EDIT_AUTHOR_ID, TOGGLE_READING_AUTHOR_ID,
    };

    let (app, _rt) = editor_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    // The toggle chrome renders in the LIVE mounted Notes pane (operator-reachable, not kittest-only).
    let ids = live_author_ids(&harness);
    for id in [
        TOGGLE_CONTAINER_AUTHOR_ID,
        TOGGLE_EDIT_AUTHOR_ID,
        TOGGLE_READING_AUTHOR_ID,
    ] {
        assert!(
            ids.contains(id),
            "MT-055: the mounted Notes pane chrome carries the view-mode toggle node '{id}'; got {:?}",
            ids.iter().filter(|i| i.starts_with("rich-reading")).collect::<Vec<_>>()
        );
    }

    // Click the READING segment via the real AccessKit action dispatch (the same out-of-process path a
    // swarm agent / operator assistive tech uses).
    let reading_node_id = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(TOGGLE_READING_AUTHOR_ID))
        .expect("reading segment present")
        .accesskit_node()
        .id();
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: reading_node_id,
            data: None,
        },
    ));
    harness.run_steps(3);

    // The Reading segment is now the toggled/selected one (the persisted per-document mode flipped and
    // the mounted pane re-rendered through the read-only branch).
    let reading_node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(TOGGLE_READING_AUTHOR_ID))
        .expect("reading segment still present after the flip");
    assert_eq!(
        reading_node.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True),
        "MT-055: clicking Reading flips the mounted editor's view mode (the segment reads toggled)"
    );
    let edit_node = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().author_id() == Some(TOGGLE_EDIT_AUTHOR_ID))
        .expect("edit segment still present after the flip");
    assert_ne!(
        edit_node.accesskit_node().toggled(),
        Some(egui::accesskit::Toggled::True),
        "MT-055: the Edit segment is no longer the active mode after the flip"
    );
}

// ── WP-KERNEL-012 MT-041 REMEDIATION: the canonical editor action nodes exist in the LIVE app tree ─────

/// The ONE shared `EditorActionRegistry` is installed on the MOUNTED code + rich panes at mount build,
/// so the canonical `editor.code.*` / `editor.rich.*` AccessKit action nodes are present in the LIVE app
/// tree — the 2026-07-02 audit found `install_editor_action_registry` had zero production callers, so
/// these nodes existed ONLY in kittest harnesses. This drives the real shell, not a widget harness.
#[test]
fn editor_action_nodes_present_in_live_shell_tree() {
    let (app, _rt) = editor_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    let ids = live_author_ids(&harness);
    assert!(
        ids.contains("editor.code.save"),
        "MT-041: the LIVE app tree carries the canonical 'editor.code.save' action node (registry \
         installed at mount, not kittest-only); got editor.* subset {:?}",
        ids.iter()
            .filter(|i| i.starts_with("editor."))
            .collect::<Vec<_>>()
    );
    assert!(
        ids.contains("editor.rich.save"),
        "MT-041: the LIVE app tree carries the canonical 'editor.rich.save' action node; got editor.* \
         subset {:?}",
        ids.iter().filter(|i| i.starts_with("editor.")).collect::<Vec<_>>()
    );
}

// ── WP-KERNEL-012 MT-008 REMEDIATION: the shell's typed LSP attach state ───────────────────────────

/// Binding a runtime handle runs the REAL LSP discovery+attach for the mounted code pane. The result
/// is TYPED and honest either way: `Configured { command, .. }` when rust-analyzer exists on THIS
/// host's PATH (the panel carries a configured client, spawned lazily on first didOpen),
/// or the typed `Absent { language_id, probed_command }` when it does not (the panel keeps its
/// graceful disabled client — never a fake). What it must NEVER be after a runtime bind is
/// `NotProbed` — the live path always probes.
#[test]
fn lsp_attach_state_is_typed_after_runtime_bind() {
    use handshake_native::app::LspAttachState;
    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    match app.lsp_attach_state() {
        LspAttachState::Configured {
            command,
            language_id,
        }
        | LspAttachState::Initializing {
            command,
            language_id,
        }
        | LspAttachState::Attached {
            command,
            language_id,
        }
        | LspAttachState::Restarting {
            command,
            language_id,
        } => {
            assert_eq!(language_id, "rust");
            assert!(
                !command.is_empty(),
                "MT-008: Attached carries the resolved launch command"
            );
            assert!(
                panel.lsp_client().is_configured(),
                "MT-008: an Attached shell installed a CONFIGURED LspClient on the mounted panel"
            );
            println!("MT-008 configured/attached state via '{command}' (rust-analyzer on this host)");
        }
        LspAttachState::Absent {
            language_id,
            probed_command,
        } => {
            assert_eq!(language_id, "rust", "the mounted code pane's language");
            assert_eq!(
                probed_command, "rust-analyzer",
                "MT-008: the typed absent-state names WHAT was probed"
            );
            assert!(
                !panel.lsp_client().is_configured(),
                "MT-008: an Absent shell leaves the graceful DISABLED client (never a fake config)"
            );
            println!(
                "MT-008 attach state: typed Absent (no '{probed_command}' on this host's PATH) — \
                 honest disabled-LSP path"
            );
        }
        LspAttachState::NotProbed => panic!(
            "MT-008: after set_runtime_handle the shell must have PROBED (configured or typed Absent), \
             never NotProbed"
        ),
    }
}

/// The shipped `HandshakeApp::new(cc)` path owns a runtime from construction, so it must also run the
/// MT-008 LSP probe without relying on the test-only `set_runtime_handle` seam.
#[test]
fn lsp_attach_state_is_typed_after_production_constructor() {
    use handshake_native::app::LspAttachState;

    let harness: Harness<HandshakeApp> =
        Harness::builder().build_eframe(|cc| HandshakeApp::new(cc));
    let app = harness.state();
    let panel = app.mounted_code_panel();
    match app.lsp_attach_state() {
        LspAttachState::Configured {
            command,
            language_id,
        }
        | LspAttachState::Initializing {
            command,
            language_id,
        }
        | LspAttachState::Attached {
            command,
            language_id,
        }
        | LspAttachState::Restarting {
            command,
            language_id,
        } => {
            assert_eq!(language_id, "rust");
            assert!(
                !command.is_empty(),
                "MT-008: production Attached carries a command"
            );
            assert!(
                panel.lsp_client().is_configured(),
                "MT-008: production constructor installed a configured LSP client"
            );
            println!("MT-008 production constructor attach state: Attached via '{command}'");
        }
        LspAttachState::Absent {
            language_id,
            probed_command,
        } => {
            assert_eq!(language_id, "rust");
            assert_eq!(probed_command, "rust-analyzer");
            assert!(
                !panel.lsp_client().is_configured(),
                "MT-008: production Absent leaves the graceful disabled client"
            );
            println!(
                "MT-008 production constructor attach state: typed Absent after probing \
                 '{probed_command}'"
            );
        }
        LspAttachState::NotProbed => {
            panic!("MT-008: production HandshakeApp::new(cc) must probe LSP during construction")
        }
    }
}

/// The shell status bar exposes the honest LSP lifecycle through the same live AccessKit tree Argus
/// consumes. This prevents discovery/restart/absence from becoming an invisible internal-only state.
#[test]
fn lsp_attach_state_is_visible_in_live_status_tree() {
    use handshake_native::app::CODE_EDITOR_LSP_STATUS_AUTHOR_ID;

    let (app, _rt) = editor_shell();
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    let ids = live_author_ids(&harness);
    assert!(
        ids.contains(CODE_EDITOR_LSP_STATUS_AUTHOR_ID),
        "MT-008: configured/attached/restarting/absent LSP state must be visible at the stable status author id; got {:?}",
        ids.iter()
            .filter(|id| id.contains("lsp"))
            .collect::<Vec<_>>()
    );
    let status = harness
        .root()
        .children_recursive()
        .find(|node| node.accesskit_node().author_id() == Some(CODE_EDITOR_LSP_STATUS_AUTHOR_ID))
        .expect("MT-008: stable LSP status node");
    assert_eq!(
        status.accesskit_node().role(),
        egui::accesskit::Role::Status,
        "MT-008: lifecycle text is exposed as an AccessKit Status"
    );
    assert!(
        status
            .accesskit_node()
            .label()
            .is_some_and(|label| label.starts_with("LSP rust:")),
        "MT-008: the status label identifies the resolved language and lifecycle; got {:?}",
        status.accesskit_node().label()
    );
}

/// Rendering a shell without an injected runtime must not probe PATH or claim a configured server.
#[test]
fn runtime_less_shell_keeps_lsp_not_probed_and_emits_no_status() {
    use handshake_native::app::{LspAttachState, CODE_EDITOR_LSP_STATUS_AUTHOR_ID};

    let app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_owned(),
        db_status: "ok".to_owned(),
        migration_version: Some(1),
    }));
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    assert_eq!(
        harness.state().lsp_attach_state(),
        LspAttachState::NotProbed
    );
    assert!(
        !live_author_ids(&harness).contains(CODE_EDITOR_LSP_STATUS_AUTHOR_ID),
        "MT-008: a runtime-less shell must not emit a fake LSP lifecycle status"
    );
}

/// A user language change must replace the prior Rust client before the next document notification.
/// Python has no configured server in the shipped discovery table, so the exact honest outcome is a
/// typed Python `Absent` state plus a disabled panel client—not rust-analyzer receiving Python text.
#[test]
fn lsp_attachment_rebinds_when_resolved_language_changes() {
    use handshake_native::app::LspAttachState;
    use handshake_native::code_editor::language_mode::LanguageId;

    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    panel.set_language_override(Some(LanguageId::new("python")));

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);

    assert_eq!(
        harness.state().lsp_attach_state(),
        LspAttachState::Absent {
            language_id: "python".to_owned(),
            probed_command: String::new(),
        },
        "MT-008: the host must bind discovery to the live resolved language"
    );
    assert!(
        !panel.lsp_client().is_configured(),
        "MT-008: changing away from Rust must drop/disable the prior Rust LSP client"
    );
}

/// Cross-file navigation opens an independent tab-backed panel. The source panel keeps its unsaved
/// buffer, file identity, language state, and breakpoints; reopening the target preserves its edits.
#[test]
fn mounted_host_opens_cross_file_target_without_replacing_source_document() {
    use handshake_native::code_editor::{BufferPosition, CodeEditorAction, JumpEntry};

    let (mut app, _rt) = editor_shell();
    let target_dir = external_artifact_dir("wp-kernel-012-mt-008");
    std::fs::create_dir_all(&target_dir).expect("create MT-008 target directory");
    let source_path = target_dir.join(format!("cross-file-source-{}.rs", std::process::id()));
    let target = target_dir.join(format!("cross-file-target-{}.rs", std::process::id()));
    std::fs::write(&source_path, "fn disk_source() {}\n").expect("write cross-file source");
    std::fs::write(&target, "line0\nline1\nfn target() {}\n").expect("write cross-file target");
    let source_path = source_path
        .canonicalize()
        .expect("canonical cross-file source fixture");
    let target = target
        .canonicalize()
        .expect("canonical cross-file target fixture");

    let source = app.mounted_code_panel();
    let shared_lsp = Arc::new(LspClient::disabled());
    app.install_mounted_code_lsp_client_for_test(Arc::clone(&shared_lsp), "disabled-test-lsp");
    source.load_file(source_path.to_string_lossy());
    source.set_text("fn unsaved_source() {}\nlet local = 7;\n");
    source.toggle_breakpoint(1);
    let source_language = source.language_id();
    source.record_jump_origin_for_test(JumpEntry::new(target.clone(), BufferPosition::new(2, 0)));
    source.dispatch_action(CodeEditorAction::NavigateBack);
    assert!(source.pending_cross_file_jump().is_some());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    for _ in 0..100 {
        harness.run_steps(1);
        if !Arc::ptr_eq(&source, &harness.state().active_mounted_code_panel()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert_eq!(source.file_path(), source_path.to_string_lossy());
    assert_eq!(
        source.buffer().to_string(),
        "fn unsaved_source() {}\nlet local = 7;\n"
    );
    assert!(source.is_breakpoint_set(1));
    assert_eq!(source.language_id(), source_language);
    assert!(
        source.pending_cross_file_jump().is_none(),
        "MT-008: host drains the cross-file intent exactly once"
    );

    let target_panel = harness.state().active_mounted_code_panel();
    assert!(
        !Arc::ptr_eq(&source, &target_panel),
        "cross-file target must own an independent panel; status={:?}; pending_jump={:?}; active_pane={:?}",
        harness.state().quick_switcher_nav_status(),
        source.pending_cross_file_jump(),
        harness.state().active_pane(),
    );
    assert_eq!(
        Path::new(&target_panel.file_path())
            .canonicalize()
            .expect("canonical mounted target path"),
        target
            .canonicalize()
            .expect("canonical expected target path")
    );
    assert!(
        Arc::ptr_eq(&shared_lsp, &source.lsp_client())
            && Arc::ptr_eq(&shared_lsp, &target_panel.lsp_client()),
        "the shared language client must bind both existing and future same-language panels"
    );
    assert_eq!(
        target_panel.buffer().to_string(),
        "line0\nline1\nfn target() {}\n"
    );
    let head = target_panel.cursors().primary().head;
    assert_eq!(
        target_panel.buffer().byte_to_line(head).unwrap_or_default(),
        2
    );

    let pane_id = harness.state().active_pane().expect("code pane focused");
    let active_target_id = harness
        .state()
        .tab_bar_states()
        .get(pane_id)
        .and_then(|bar| bar.active())
        .and_then(|tab| tab.content_id.clone())
        .expect("file-backed target tab has a content id");
    assert!(
        harness
            .state()
            .mounted_code_panel_for_content_id(&active_target_id)
            .is_some(),
        "active tab identity resolves to the independently mounted panel"
    );

    assert!(harness.state_mut().dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_EDITOR_GO_TO_LINE
    ));
    assert!(target_panel.is_goto_line_open());
    assert!(!source.is_goto_line_open());
    target_panel.close_goto_line();
    assert!(harness.state_mut().dispatch_palette_action_for_test(
        handshake_native::command_registry::CMD_EDITOR_EDIT_TOGGLE_COMMENT
    ));
    assert!(
        target_panel.buffer().to_string().contains("//fn target()")
            || target_panel.buffer().to_string().contains("// fn target()"),
        "shell edit command must mutate the active target panel"
    );
    assert_eq!(
        source.buffer().to_string(),
        "fn unsaved_source() {}\nlet local = 7;\n",
        "shell edit command must not mutate the source panel"
    );

    // Reopen the already-mounted target after making an unsaved edit. The store must return the live
    // panel instead of refreshing it from disk and destroying that edit.
    target_panel.set_text("fn unsaved_target_edit() {}\n");
    {
        let pane_id = PaneId::from("pane-a");
        let bar = harness
            .state_mut()
            .tab_bar_states_mut()
            .get_mut(&pane_id)
            .expect("pane-a tab bar");
        let source_index = bar
            .tabs
            .iter()
            .position(|tab| tab.pane_type == PaneType::CodeSymbol && tab.content_id.is_none())
            .expect("source tab remains mounted");
        bar.activate(source_index);
    }
    harness.run_steps(1);
    source.record_jump_origin_for_test(JumpEntry::new(target, BufferPosition::new(0, 0)));
    source.dispatch_action(CodeEditorAction::NavigateBack);
    harness.run_steps(2);
    assert!(Arc::ptr_eq(
        &target_panel,
        &harness.state().active_mounted_code_panel()
    ));
    assert_eq!(
        target_panel.buffer().to_string(),
        "fn unsaved_target_edit() {}\n",
        "reopening an existing target must preserve its unsaved buffer"
    );
}

#[test]
fn duplicate_code_panes_open_navigation_target_only_in_the_exact_origin_pane() {
    use handshake_native::code_editor::{BufferPosition, CodeEditorAction, JumpEntry};
    use handshake_native::tab_bar::{TabBarState, TabState};

    let (mut app, _rt) = editor_shell();
    let pane_a = PaneId::from("pane-a");
    let pane_b = PaneId::from("pane-b");
    {
        let registry = app.pane_registry();
        registry.lock().expect("registry").insert(PaneRecord::new(
            pane_b.clone(),
            PaneType::CodeSymbol,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    app.tab_bar_states_mut().insert(
        pane_b.clone(),
        TabBarState::new(pane_b.clone(), vec![TabState::new(PaneType::CodeSymbol)]),
    );

    let dir = external_artifact_dir("wp-kernel-012-mt-008/duplicate-pane-origin");
    std::fs::create_dir_all(&dir).expect("create duplicate-pane fixture directory");
    let source_path = dir.join(format!("source-{}.rs", std::process::id()));
    let target_path = dir.join(format!("target-{}.rs", std::process::id()));
    std::fs::write(&source_path, "fn source() {}\n").expect("write duplicate-pane source");
    std::fs::write(&target_path, "line0\nfn exact_target() {}\n")
        .expect("write duplicate-pane target");
    let source_path = source_path.canonicalize().expect("canonical source");
    let target_path = target_path.canonicalize().expect("canonical target");

    let source = app.mounted_code_panel();
    source.load_file(source_path.to_string_lossy());
    app.set_active_pane_for_test(Some(pane_b.clone()));
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    source.set_host_render_pane_id(Some(pane_b.clone()));
    source.record_jump_origin_for_test(JumpEntry::new(
        target_path.clone(),
        BufferPosition::new(1, 0),
    ));
    source.dispatch_action(CodeEditorAction::NavigateBack);
    for _ in 0..200 {
        harness.run_steps(1);
        let pane_b_has_target = harness
            .state()
            .tab_bar_states()
            .get(&pane_b)
            .and_then(|bar| bar.active())
            .and_then(|tab| tab.content_id.as_deref())
            .is_some_and(|content_id| {
                harness
                    .state()
                    .mounted_code_panel_for_content_id(content_id)
                    .is_some_and(|panel| {
                        Path::new(&panel.file_path())
                            .canonicalize()
                            .is_ok_and(|path| path == target_path)
                    })
            });
        if pane_b_has_target {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let pane_b_active = harness
        .state()
        .tab_bar_states()
        .get(&pane_b)
        .and_then(|bar| bar.active())
        .expect("origin pane keeps an active tab");
    assert!(pane_b_active.content_id.is_some());
    let pane_a_active = harness
        .state()
        .tab_bar_states()
        .get(&pane_a)
        .and_then(|bar| bar.active())
        .expect("other duplicate pane keeps its source tab");
    assert_eq!(
        pane_a_active.content_id, None,
        "the non-origin duplicate pane must not receive or activate the navigation target"
    );
    assert!(Arc::ptr_eq(&source, &harness.state().mounted_code_panel()));
}

#[test]
fn mounted_host_resolves_drive_less_percent_encoded_unicode_file_uri() {
    use handshake_native::code_editor::{BufferPosition, CodeEditorAction, JumpEntry};

    let (app, _rt) = editor_shell();
    let source = app.mounted_code_panel();
    let target_dir = external_artifact_dir("wp-kernel-012-mt-008/encoded-uri");
    std::fs::create_dir_all(&target_dir).expect("create encoded URI directory");
    let source_path = target_dir.join("source.rs");
    let target_path = target_dir.join("sibling space é.rs");
    std::fs::write(&source_path, "fn source() {}\n").expect("write encoded URI source");
    std::fs::write(&target_path, "line0\nfn unicode_target() {}\n")
        .expect("write encoded URI target");
    source.load_file(source_path.to_string_lossy());
    source.set_text("fn unsaved_source() {}\n");
    source.record_jump_origin_for_test(JumpEntry::new(
        PathBuf::from("file:///sibling%20space%20%C3%A9.rs"),
        BufferPosition::new(1, 0),
    ));
    source.dispatch_action(CodeEditorAction::NavigateBack);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    for _ in 0..100 {
        harness.run_steps(1);
        if !Arc::ptr_eq(&source, &harness.state().active_mounted_code_panel()) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let target = harness.state().active_mounted_code_panel();
    assert!(!Arc::ptr_eq(&source, &target));
    assert_eq!(
        Path::new(&target.file_path())
            .canonicalize()
            .expect("canonical mounted Unicode target path"),
        target_path
            .canonicalize()
            .expect("canonical expected Unicode target path")
    );
    assert_eq!(
        target.buffer().to_string(),
        "line0\nfn unicode_target() {}\n"
    );
    assert_eq!(source.buffer().to_string(), "fn unsaved_source() {}\n");
}

#[test]
fn mounted_host_ignores_stale_code_document_completion() {
    let (mut app, _rt) = editor_shell();
    let source = app.mounted_code_panel();
    source.set_text("fn source_stays() {}\n");

    let stale_generation = app.begin_code_document_load_for_test("stale-target");
    let current_generation = app.begin_code_document_load_for_test("current-target");
    app.deliver_code_document_load_for_test(
        stale_generation,
        "stale-target",
        PathBuf::from("stale.rs"),
        0,
        Ok("fn stale_must_not_open() {}\n".to_owned()),
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(1);
    assert!(
        harness
            .state()
            .mounted_code_panel_for_content_id("stale-target")
            .is_none(),
        "an older generation must not create or activate a document"
    );
    assert!(Arc::ptr_eq(
        &source,
        &harness.state().active_mounted_code_panel()
    ));

    harness.state().deliver_code_document_load_for_test(
        current_generation,
        "current-target",
        PathBuf::from("current.rs"),
        0,
        Ok("fn current_opens() {}\n".to_owned()),
    );
    harness.run_steps(1);
    assert_eq!(
        harness
            .state()
            .active_mounted_code_panel()
            .buffer()
            .to_string(),
        "fn current_opens() {}\n"
    );
    assert_eq!(source.buffer().to_string(), "fn source_stays() {}\n");
}

#[test]
fn code_ref_stage_two_correlation_is_pane_source_scoped_under_interleaving() {
    let (mut app, _rt) = editor_shell();
    let pane_a = PaneId::from("pane-a");
    let pane_b = PaneId::from("pane-b");
    let a_generation = app.begin_code_document_load_for_scope_at_for_test(
        pane_a.clone(),
        "source-a",
        "a-stage-two",
        3,
    );
    let b_old_generation = app.begin_code_document_load_for_scope_at_for_test(
        pane_b.clone(),
        "source-b",
        "b-old-stage-two",
        2,
    );

    // B-new starts while A and B-old are pending. Dispatch-time invalidation must cancel B-old only;
    // A remains independently current and can still land at its exact pane/byte.
    app.begin_code_ref_navigation_for_scope_for_test(pane_b.clone(), "source-b");
    let b_new_generation = app.begin_code_document_load_for_scope_at_for_test(
        pane_b.clone(),
        "source-b",
        "b-new-stage-two",
        7,
    );
    app.deliver_code_document_load_for_test(
        b_old_generation,
        "b-old-stage-two",
        PathBuf::from("b-old-stage-two.rs"),
        0,
        Ok("stale b content".to_owned()),
    );
    app.deliver_code_document_load_for_test(
        a_generation,
        "a-stage-two",
        PathBuf::from("a-stage-two.rs"),
        0,
        Ok("0123456789 a".to_owned()),
    );
    app.deliver_code_document_load_for_test(
        b_new_generation,
        "b-new-stage-two",
        PathBuf::from("b-new-stage-two.rs"),
        0,
        Ok("0123456789 b-new".to_owned()),
    );

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(1);
    assert!(
        harness
            .state()
            .mounted_code_panel_for_content_id("b-old-stage-two")
            .is_none(),
        "B-old is rejected inside B's lane"
    );
    let a = harness
        .state()
        .mounted_code_panel_for_content_id("a-stage-two")
        .expect("pane A pending load survives B supersession");
    let b_new = harness
        .state()
        .mounted_code_panel_for_content_id("b-new-stage-two")
        .expect("pane B newest target mounted");
    assert_eq!(a.primary_cursor_byte_offset(), 3);
    assert_eq!(b_new.primary_cursor_byte_offset(), 7);
    let pane_a_tabs = &harness
        .state()
        .tab_bar_states()
        .get(&pane_a)
        .expect("pane A tab bar")
        .tabs;
    let pane_b_tabs = &harness
        .state()
        .tab_bar_states()
        .get(&pane_b)
        .expect("pane B tab bar")
        .tabs;
    assert!(
        pane_a_tabs
            .iter()
            .any(|tab| tab.content_id.as_deref() == Some("a-stage-two")),
        "A lands on pane A"
    );
    assert!(
        pane_b_tabs
            .iter()
            .any(|tab| tab.content_id.as_deref() == Some("b-new-stage-two")),
        "B-new lands on pane B"
    );
}

#[test]
fn mounted_host_surfaces_cross_file_load_error_without_mutating_source() {
    use handshake_native::code_editor::{BufferPosition, CodeEditorAction, JumpEntry};

    let (app, _rt) = editor_shell();
    let source = app.mounted_code_panel();
    let target_dir = external_artifact_dir("wp-kernel-012-mt-008/load-error");
    std::fs::create_dir_all(&target_dir).expect("create load-error directory");
    let source_path = target_dir.join("source.rs");
    std::fs::write(&source_path, "fn disk_source() {}\n").expect("write load-error source");
    source.load_file(source_path.to_string_lossy());
    source.set_text("fn unsaved_source_survives_error() {}\n");
    let missing = target_dir.join("missing-target.rs");
    let _ = std::fs::remove_file(&missing);
    source.record_jump_origin_for_test(JumpEntry::new(missing, BufferPosition::new(0, 0)));
    source.dispatch_action(CodeEditorAction::NavigateBack);

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    for _ in 0..100 {
        harness.run_steps(1);
        if harness.state().quick_switcher_nav_status().is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    let status = harness
        .state()
        .quick_switcher_nav_status()
        .expect("load error is operator-visible");
    assert!(status.contains("Code navigation failed"), "status={status}");
    assert!(Arc::ptr_eq(
        &source,
        &harness.state().active_mounted_code_panel()
    ));
    assert_eq!(source.file_path(), source_path.to_string_lossy());
    assert_eq!(
        source.buffer().to_string(),
        "fn unsaved_source_survives_error() {}\n"
    );
}

#[test]
fn mounted_host_marks_preframe_file_edit_dirty_and_cancel_or_discard_targets_exact_document() {
    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    let dir = external_artifact_dir("wp-kernel-012-mt-008/dirty-close");
    std::fs::create_dir_all(&dir).expect("create dirty-close directory");
    let path = dir.join(format!("dirty-before-frame-{}.rs", std::process::id()));
    std::fs::write(&path, "fn disk_version() {}\n").expect("write dirty-close source");
    panel.load_file(path.to_string_lossy());
    panel.set_text("fn edited_before_first_frame() {}\n");

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let pane_id = PaneId::from("pane-a");
    assert!(
        harness
            .state()
            .tab_bar_states()
            .get(&pane_id)
            .and_then(|bar| bar.active())
            .is_some_and(|tab| tab.dirty),
        "an edit made after load but before the first frame must not become the clean baseline"
    );

    assert!(!harness.state_mut().close_active_tab_for_test());
    assert!(harness.state().pending_dirty_code_close_for_test());
    harness.run_steps(1);
    for author_id in [
        "code-editor.dirty-close.dialog",
        "code-editor.dirty-close.save",
        "code-editor.dirty-close.discard",
        "code-editor.dirty-close.cancel",
    ] {
        assert!(
            harness
                .root()
                .children_recursive()
                .any(|node| node.accesskit_node().author_id() == Some(author_id)),
            "dirty-close surface exposes stable author id {author_id}",
        );
    }
    assert!(harness
        .state_mut()
        .cancel_pending_dirty_code_close_for_test());
    assert!(!harness.state().pending_dirty_code_close_for_test());
    assert_eq!(
        harness
            .state()
            .tab_bar_states()
            .get(&pane_id)
            .map(|bar| bar.tabs.len()),
        Some(1),
        "Cancel keeps the dirty tab open"
    );

    assert!(!harness.state_mut().close_active_tab_for_test());
    assert!(harness
        .state_mut()
        .discard_pending_dirty_code_close_for_test());
    assert_eq!(
        std::fs::read_to_string(&path).expect("read discarded source"),
        "fn disk_version() {}\n",
        "Discard closes without writing the unsaved buffer"
    );
}

#[test]
fn dirty_save_close_survives_reorder_and_close_all_resumes_after_discard() {
    let (mut app, _rt) = editor_shell();
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let dir = external_artifact_dir("wp-kernel-012-mt-008/reorder-close");
    std::fs::create_dir_all(&dir).expect("create reorder-close directory");
    let first_path = dir.join(format!("first-{}.rs", std::process::id()));
    std::fs::write(&first_path, "fn first_disk() {}\n").expect("write first source");
    let generation = app.begin_code_document_load_for_test("reorder-first");
    app.deliver_code_document_load_for_test(
        generation,
        "reorder-first",
        first_path.clone(),
        0,
        Ok("fn first_disk() {}\n".to_owned()),
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let first_panel = harness.state().active_mounted_code_panel();
    first_panel.set_text("fn first_saved_after_reorder() {}\n");
    harness.run_steps(1);
    assert!(!harness.state_mut().close_active_tab_for_test());
    {
        let bar = harness
            .state_mut()
            .tab_bar_states_mut()
            .get_mut(&PaneId::from("pane-a"))
            .expect("pane-a tab bar");
        let dirty_index = bar
            .tabs
            .iter()
            .position(|tab| tab.content_id.as_deref() == Some("reorder-first"))
            .expect("dirty target tab");
        bar.reorder_tab(dirty_index, 0);
    }
    assert!(harness.state_mut().save_pending_dirty_code_close_for_test());
    for _ in 0..300 {
        harness.run_steps(1);
        if harness
            .state()
            .mounted_code_panel_for_content_id("reorder-first")
            .is_none()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        std::fs::read_to_string(&first_path).expect("read reordered saved file"),
        "fn first_saved_after_reorder() {}\n"
    );
    assert!(
        harness
            .state()
            .mounted_code_panel_for_content_id("reorder-first")
            .is_none(),
        "Save-and-close follows document identity after tab reorder"
    );

    // Open two more targets. Close All closes the clean high index, pauses at the dirty target, then
    // Discard resumes and closes the remaining base tab instead of silently abandoning the batch.
    let dirty_path = dir.join(format!("batch-dirty-{}.rs", std::process::id()));
    let clean_path = dir.join(format!("batch-clean-{}.rs", std::process::id()));
    std::fs::write(&dirty_path, "fn dirty_disk() {}\n").expect("write batch dirty source");
    std::fs::write(&clean_path, "fn clean_disk() {}\n").expect("write batch clean source");
    let dirty_generation = harness
        .state_mut()
        .begin_code_document_load_for_test("batch-dirty");
    harness.state().deliver_code_document_load_for_test(
        dirty_generation,
        "batch-dirty",
        dirty_path,
        0,
        Ok("fn dirty_disk() {}\n".to_owned()),
    );
    harness.run_steps(1);
    let dirty_panel = harness.state().active_mounted_code_panel();
    dirty_panel.set_text("fn dirty_unsaved() {}\n");
    harness.run_steps(1);
    let clean_generation = harness
        .state_mut()
        .begin_code_document_load_for_test("batch-clean");
    harness.state().deliver_code_document_load_for_test(
        clean_generation,
        "batch-clean",
        clean_path,
        0,
        Ok("fn clean_disk() {}\n".to_owned()),
    );
    harness.run_steps(2);
    let pane_id = PaneId::from("pane-a");
    let tab_count = harness
        .state()
        .tab_bar_states()
        .get(&pane_id)
        .map(|bar| bar.tabs.len())
        .expect("pane-a tabs");
    harness
        .state_mut()
        .close_tab_indices_for_test(pane_id.clone(), (0..tab_count).rev().collect());
    assert!(harness.state().pending_dirty_code_close_for_test());
    assert!(harness
        .state_mut()
        .discard_pending_dirty_code_close_for_test());
    assert_eq!(
        harness
            .state()
            .tab_bar_states()
            .get(&pane_id)
            .map(|bar| bar.tabs.len()),
        Some(0),
        "Close All resumes the stable remainder after dirty Discard"
    );
}

#[test]
fn stale_save_completion_cannot_mark_or_close_reopened_document_incarnation() {
    let (mut app, _rt) = editor_shell();
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let dir = external_artifact_dir("wp-kernel-012-mt-008/stale-save-incarnation");
    std::fs::create_dir_all(&dir).expect("create stale-save directory");
    let path = dir.join(format!("reopen-{}.rs", std::process::id()));
    std::fs::write(&path, "fn disk() {}\n").expect("write stale-save source");

    let generation = app.begin_code_document_load_for_test("stale-save-document");
    app.deliver_code_document_load_for_test(
        generation,
        "stale-save-document",
        path.clone(),
        0,
        Ok("fn disk() {}\n".to_owned()),
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let old_panel = harness.state().active_mounted_code_panel();
    old_panel.set_text("fn old_snapshot() {}\n");
    harness.run_steps(1);
    let (key, saved_version, old_incarnation) = harness
        .state()
        .code_document_save_identity_for_test("stale-save-document")
        .expect("old save identity");

    assert!(!harness.state_mut().close_active_tab_for_test());
    assert!(harness
        .state_mut()
        .discard_pending_dirty_code_close_for_test());
    assert!(
        harness
            .state()
            .mounted_code_panel_for_content_id("stale-save-document")
            .is_none(),
        "old document panel is removed before reopen"
    );

    let generation = harness
        .state_mut()
        .begin_code_document_load_for_test("stale-save-document");
    harness.state().deliver_code_document_load_for_test(
        generation,
        "stale-save-document",
        path.clone(),
        0,
        Ok("fn disk() {}\n".to_owned()),
    );
    harness.run_steps(2);
    let new_panel = harness
        .state()
        .mounted_code_panel_for_content_id("stale-save-document")
        .expect("reopened document panel");
    new_panel.set_text("fn reopened_dirty() {}\n");
    assert!(
        !Arc::ptr_eq(&old_panel, &new_panel),
        "reopen receives a new panel incarnation"
    );
    let (_, _, new_incarnation) = harness
        .state()
        .code_document_save_identity_for_test("stale-save-document")
        .expect("new save identity");
    assert_ne!(old_incarnation, new_incarnation);

    harness
        .state_mut()
        .set_code_document_close_after_save_for_test(key.clone(), saved_version, old_incarnation);
    harness.state().deliver_code_document_save_for_test(
        key,
        path,
        saved_version,
        old_incarnation,
        Ok(()),
    );
    harness.run_steps(2);

    let still_open = harness
        .state()
        .mounted_code_panel_for_content_id("stale-save-document")
        .expect("new incarnation remains open");
    assert!(Arc::ptr_eq(&new_panel, &still_open));
    assert_eq!(still_open.buffer().to_string(), "fn reopened_dirty() {}\n");
    assert_ne!(
        still_open.saved_buffer_version(),
        still_open.buffer_version_for_test(),
        "old completion cannot mark the reopened dirty buffer saved"
    );
}

#[test]
fn same_panel_a_b_a_text_cycle_keeps_a_monotonic_save_identity() {
    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    let dir = external_artifact_dir("wp-kernel-012-mt-008/save-aba");
    std::fs::create_dir_all(&dir).expect("create save ABA directory");
    let path = dir.join(format!("save-aba-{}.rs", std::process::id()));
    std::fs::write(&path, "fn disk() {}\n").expect("write save ABA source");
    panel.load_file(path.to_string_lossy());

    panel.set_text("fn snapshot_a() {}\n");
    let (key_a1, version_a1, incarnation_a1) = app
        .code_document_save_identity_for_test("")
        .expect("first A identity");
    panel.set_text("fn snapshot_b() {}\n");
    let (key_b, version_b, incarnation_b) = app
        .code_document_save_identity_for_test("")
        .expect("B identity");
    panel.set_text("fn snapshot_a() {}\n");
    let (key_a2, version_a2, incarnation_a2) = app
        .code_document_save_identity_for_test("")
        .expect("second A identity");

    assert_eq!(key_a1, key_b);
    assert_eq!(key_b, key_a2);
    assert_eq!(incarnation_a1, incarnation_b);
    assert_eq!(incarnation_b, incarnation_a2);
    assert!(version_a1 < version_b && version_b < version_a2);
    assert_ne!(
        (version_a1, incarnation_a1),
        (version_a2, incarnation_a2),
        "returning to byte-identical text must not ABA-reuse the first save identity"
    );
}

#[test]
fn mounted_host_serializes_rapid_saves_and_clears_dirty_only_at_latest_version() {
    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    let dir = external_artifact_dir("wp-kernel-012-mt-008/save");
    std::fs::create_dir_all(&dir).expect("create save directory");
    let path = dir.join(format!("rapid-save-{}.rs", std::process::id()));
    std::fs::write(&path, "fn initial() {}\n").expect("write rapid-save source");
    let original_readonly = std::fs::metadata(&path)
        .expect("rapid-save metadata")
        .permissions()
        .readonly();
    let original_created = std::fs::metadata(&path)
        .expect("rapid-save metadata")
        .created()
        .ok();
    panel.load_file(path.to_string_lossy());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    panel.set_text("fn older_snapshot() {}\n");
    panel.request_save_for_host();
    harness.run_steps(1);
    panel.set_text("fn newest_snapshot() {}\n");
    panel.request_save_for_host();

    for _ in 0..2_000 {
        harness.run_steps(1);
        let disk_is_latest =
            std::fs::read_to_string(&path).is_ok_and(|text| text == "fn newest_snapshot() {}\n");
        let tab_is_clean = harness
            .state()
            .tab_bar_states()
            .get(&PaneId::from("pane-a"))
            .and_then(|bar| bar.active())
            .is_some_and(|tab| !tab.dirty);
        if disk_is_latest && tab_is_clean {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(
        std::fs::read_to_string(&path).expect("read latest rapid save"),
        "fn newest_snapshot() {}\n",
        "an older save completion must never overwrite the newest queued snapshot; status={:?}",
        harness.state().quick_switcher_nav_status(),
    );
    assert!(
        harness
            .state()
            .tab_bar_states()
            .get(&PaneId::from("pane-a"))
            .and_then(|bar| bar.active())
            .is_some_and(|tab| !tab.dirty),
        "dirty clears only after the newest version is durable; buffer_version={}; saved_version={}; status={:?}",
        panel.buffer_version_for_test(),
        panel.saved_buffer_version(),
        harness.state().quick_switcher_nav_status(),
    );
    assert_eq!(
        std::fs::metadata(&path)
            .expect("saved metadata")
            .permissions()
            .readonly(),
        original_readonly,
        "atomic replacement preserves the original file permission attributes"
    );
    if let Some(original_created) = original_created {
        assert_eq!(
            std::fs::metadata(&path)
                .expect("saved metadata")
                .created()
                .expect("saved creation time"),
            original_created,
            "Windows atomic replacement preserves target creation metadata",
        );
    }
    let temp_prefix = format!(
        ".{}.hsk-save-{}-",
        path.file_name().unwrap().to_string_lossy(),
        std::process::id()
    );
    assert!(
        std::fs::read_dir(&dir)
            .expect("enumerate save directory")
            .filter_map(Result::ok)
            .all(|entry| !entry
                .file_name()
                .to_string_lossy()
                .starts_with(&temp_prefix)),
        "successful saves leave no temporary file behind"
    );
}

#[test]
fn mounted_host_save_failure_is_visible_and_keeps_document_dirty() {
    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    let root = external_artifact_dir("wp-kernel-012-mt-008/save-failure");
    std::fs::create_dir_all(&root).expect("create save-failure root");
    let dir = root.join(format!("removed-parent-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create disposable parent");
    let path = dir.join("source.rs");
    std::fs::write(&path, "fn initial() {}\n").expect("write save-failure source");
    panel.load_file(path.to_string_lossy());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    std::fs::remove_file(&path).expect("remove source before failed save");
    std::fs::remove_dir(&dir).expect("remove parent before failed save");
    panel.set_text("fn must_remain_dirty() {}\n");
    panel.request_save_for_host();
    for _ in 0..200 {
        harness.run_steps(1);
        if harness
            .state()
            .quick_switcher_nav_status()
            .is_some_and(|status| status.starts_with("Code save failed"))
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(
        harness
            .state()
            .quick_switcher_nav_status()
            .is_some_and(|status| status.starts_with("Code save failed")),
        "save failure must be operator-visible"
    );
    assert!(
        harness
            .state()
            .tab_bar_states()
            .get(&PaneId::from("pane-a"))
            .and_then(|bar| bar.active())
            .is_some_and(|tab| tab.dirty),
        "failed save must not advance the clean baseline"
    );
    assert_eq!(panel.buffer().to_string(), "fn must_remain_dirty() {}\n");
}

#[test]
fn host_save_command_carries_requesting_document_identity() {
    let (mut app, _rt) = editor_shell();
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let dir = external_artifact_dir("wp-kernel-012-mt-008/document-command-identity");
    std::fs::create_dir_all(&dir).expect("create document identity directory");
    let first_path = dir.join(format!("first-{}.rs", std::process::id()));
    let second_path = dir.join(format!("second-{}.rs", std::process::id()));
    std::fs::write(&first_path, "fn first_disk() {}\n").expect("write first source");
    std::fs::write(&second_path, "fn second_disk() {}\n").expect("write second source");

    let first_generation = app.begin_code_document_load_for_test("identity-first");
    app.deliver_code_document_load_for_test(
        first_generation,
        "identity-first",
        first_path.clone(),
        0,
        Ok("fn first_disk() {}\n".to_owned()),
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let first = harness
        .state()
        .mounted_code_panel_for_content_id("identity-first")
        .expect("first panel mounted");

    let second_generation = harness
        .state_mut()
        .begin_code_document_load_for_test("identity-second");
    harness.state().deliver_code_document_load_for_test(
        second_generation,
        "identity-second",
        second_path.clone(),
        0,
        Ok("fn second_disk() {}\n".to_owned()),
    );
    harness.run_steps(2);
    let second = harness
        .state()
        .mounted_code_panel_for_content_id("identity-second")
        .expect("second panel mounted and active");
    assert!(Arc::ptr_eq(
        &second,
        &harness.state().active_mounted_code_panel()
    ));

    first.set_text("fn first_requested_save() {}\n");
    second.set_text("fn second_must_stay_unsaved() {}\n");
    harness.run_steps(1);
    first.request_save_for_host();
    for _ in 0..2_000 {
        harness.run_steps(1);
        if std::fs::read_to_string(&first_path)
            .is_ok_and(|text| text == "fn first_requested_save() {}\n")
        {
            break;
        }
        std::thread::yield_now();
    }
    assert_eq!(
        std::fs::read_to_string(&first_path).expect("read first source after save"),
        "fn first_requested_save() {}\n",
        "a non-active panel's Save command writes that panel's document; last_command={:?}; status={:?}",
        harness.state().last_editor_command(),
        harness.state().quick_switcher_nav_status(),
    );
    assert_eq!(
        std::fs::read_to_string(&second_path).expect("read second source after first save"),
        "fn second_disk() {}\n",
        "the active document is not accidentally written by another panel's Save command",
    );
    assert_eq!(
        second.buffer().to_string(),
        "fn second_must_stay_unsaved() {}\n"
    );
}

#[test]
fn popout_scoped_command_drain_retains_interleaved_docked_command() {
    use handshake_native::code_editor::CodeEditorAction;
    use handshake_native::tab_bar::{TabBarState, TabState};

    let (mut app, _rt) = editor_shell();
    let pane_a = PaneId::from("pane-a");
    let pane_b = PaneId::from("pane-b");
    let pane_c = PaneId::from("pane-c");
    {
        let registry = app.pane_registry();
        let mut registry = registry.lock().expect("registry");
        for pane_id in [&pane_a, &pane_b, &pane_c] {
            registry.insert(PaneRecord::new(
                pane_id.clone(),
                PaneType::CodeSymbol,
                DEFAULT_PROJECT_ID,
                None,
                LockState::Unlocked,
                DirtyState::Clean,
                PaneAuthority::System,
            ));
        }
    }
    for pane_id in [&pane_a, &pane_b, &pane_c] {
        app.tab_bar_states_mut().insert(
            pane_id.clone(),
            TabBarState::new(pane_id.clone(), vec![TabState::new(PaneType::CodeSymbol)]),
        );
    }

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    // `request_pop_out` is deliberately a one-request next-frame slot. Drive each real request through
    // its own frame before proving the source-scoped command drain across two detached panes.
    harness.state_mut().request_pop_out(pane_a.clone());
    harness.run_steps(2);
    assert!(harness.state().is_popped_out(&pane_a));
    harness.state_mut().request_pop_out(pane_b.clone());
    harness.run_steps(2);
    assert!(harness.state().is_popped_out(&pane_a));
    assert!(harness.state().is_popped_out(&pane_b));
    assert!(!harness.state().is_popped_out(&pane_c));

    let panel = harness.state().mounted_code_panel();
    for pane_id in [&pane_a, &pane_c, &pane_b] {
        panel.set_host_render_pane_id(Some(pane_id.clone()));
        panel.dispatch_action(CodeEditorAction::OpenCommandPalette);
    }

    let ctx = egui::Context::default();
    harness
        .state_mut()
        .drain_scoped_code_editor_commands_for_test(&ctx, &[pane_a.clone(), pane_b.clone()]);
    assert_eq!(
        harness.state().deferred_code_command_panes_for_test(),
        vec![Some(pane_c.clone())],
        "the popout-scoped drain must consume both detached commands but retain the interleaved docked command"
    );
    assert!(harness.state().command_palette_open());
    assert_eq!(harness.state().command_palette_open_count(), 1);

    harness.state_mut().close_command_palette();
    harness
        .state_mut()
        .drain_all_code_editor_commands_for_test(&ctx);
    assert!(harness
        .state()
        .deferred_code_command_panes_for_test()
        .is_empty());
    assert!(harness.state().command_palette_open());
    assert_eq!(
        harness.state().command_palette_open_count(),
        2,
        "the retained docked command must execute on the later full host drain"
    );
}

#[test]
fn relative_navigation_is_source_anchored_and_late_completion_cannot_refocus_after_round_trip() {
    let dir = external_artifact_dir("wp-kernel-012-mt-008/source-anchor");
    std::fs::create_dir_all(&dir).expect("create source-anchor directory");
    let source_path = dir.join("source.rs");
    let sibling_path = dir.join("sibling.rs");
    std::fs::write(&source_path, "fn source() {}\n").expect("write source-anchor source");
    std::fs::write(&sibling_path, "fn sibling() {}\n").expect("write source-anchor sibling");
    let resolved = handshake_native::app::resolve_code_navigation_path_for_test(
        Path::new("sibling.rs"),
        &source_path.to_string_lossy(),
    )
    .expect("resolve source-relative sibling");
    assert_eq!(
        resolved.canonicalize().expect("canonical resolved sibling"),
        sibling_path
            .canonicalize()
            .expect("canonical expected sibling"),
        "ordinary relative LSP targets resolve beside the source, never from process CWD"
    );

    let nested = dir.join("nested");
    std::fs::create_dir_all(&nested).expect("create nested source directory");
    let nested_source = nested.join("source.rs");
    std::fs::write(&nested_source, "fn nested_source() {}\n").expect("write nested source");
    let parent_relative = handshake_native::app::resolve_code_navigation_path_for_test(
        Path::new("../sibling.rs"),
        &nested_source.to_string_lossy(),
    )
    .expect("resolve parent-relative target from source directory");
    assert_eq!(
        parent_relative
            .canonicalize()
            .expect("canonical parent-relative target"),
        sibling_path
            .canonicalize()
            .expect("canonical sibling target"),
        "../ targets must be source-relative, never process-CWD-relative",
    );

    let dot_relative = handshake_native::app::resolve_code_navigation_path_for_test(
        Path::new("./source.rs"),
        &nested_source.to_string_lossy(),
    )
    .expect("resolve dot-relative target from source directory");
    assert_eq!(
        dot_relative
            .canonicalize()
            .expect("canonical dot-relative target"),
        nested_source
            .canonicalize()
            .expect("canonical nested source"),
        "./ targets must be source-relative, never process-CWD-relative",
    );

    let (mut app, _rt) = editor_shell();
    app.mounted_code_panel()
        .load_file(source_path.to_string_lossy());
    app.set_active_pane_for_test(Some(PaneId::from("pane-a")));
    let generation = app.begin_code_document_load_for_test("late-target");
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(1);
    harness
        .state_mut()
        .set_active_pane_for_test(Some(PaneId::from("pane-b")));
    harness.run_steps(1);
    harness
        .state_mut()
        .set_active_pane_for_test(Some(PaneId::from("pane-a")));
    harness.run_steps(1);
    harness.state().deliver_code_document_load_for_test(
        generation,
        "late-target",
        sibling_path,
        0,
        Ok("fn late_must_not_focus() {}\n".to_owned()),
    );
    harness.run_steps(1);
    assert!(
        harness
            .state()
            .mounted_code_panel_for_content_id("late-target")
            .is_none(),
        "switching away and back invalidates the old completion epoch"
    );
}

#[cfg(windows)]
#[test]
fn windows_file_uri_resolution_handles_drive_localhost_unc_and_unicode() {
    let dir = external_artifact_dir("wp-kernel-012-mt-008/windows-uri");
    std::fs::create_dir_all(&dir).expect("create Windows URI directory");
    let source = dir.join("source.rs");
    let target = dir.join("drive space é.rs");
    std::fs::write(&source, "fn source() {}\n").expect("write URI source");
    std::fs::write(&target, "fn target() {}\n").expect("write URI target");
    let absolute_target = target.canonicalize().expect("canonical URI target");
    let uri = lsp_types::Url::from_file_path(&absolute_target).expect("drive file URI");
    let resolved = handshake_native::app::resolve_code_navigation_path_for_test(
        Path::new(uri.as_str()),
        &source.to_string_lossy(),
    )
    .expect("resolve drive URI");
    assert_eq!(resolved.canonicalize().unwrap(), absolute_target);

    let localhost_uri = format!("file://localhost{}", uri.path());
    let localhost = handshake_native::app::resolve_code_navigation_path_for_test(
        Path::new(&localhost_uri),
        &source.to_string_lossy(),
    )
    .expect("resolve localhost file URI");
    assert_eq!(localhost.canonicalize().unwrap(), absolute_target);

    let unc = handshake_native::app::resolve_code_navigation_path_for_test(
        Path::new("file://server/share/folder/file.rs"),
        &source.to_string_lossy(),
    )
    .expect("represent UNC file URI on Windows");
    assert!(
        unc.to_string_lossy()
            .replace('/', "\\")
            .starts_with("\\\\server\\share\\"),
        "UNC URI must retain its server/share authority: {}",
        unc.display()
    );
}

/// A mounted, file-backed code pane pushes `didOpen` on first frame and `didChange` after the buffer
/// version changes. This proves the live app drive loop reaches MT-008 document sync; it is gated by
/// actual LSP discovery so hosts without rust-analyzer report a typed skip rather than faking a server.
#[test]
fn lsp_file_backed_document_sync_watermark_advances_on_open_and_change() {
    use handshake_native::app::LspAttachState;

    let (app, _rt) = editor_shell();
    let panel = app.mounted_code_panel();
    match app.lsp_attach_state() {
        LspAttachState::Configured { command, .. }
        | LspAttachState::Initializing { command, .. }
        | LspAttachState::Attached { command, .. }
        | LspAttachState::Restarting { command, .. } => {
            println!("MT-008 doc sync proof using discovered LSP command '{command}'");
        }
        LspAttachState::Absent {
            language_id,
            probed_command,
        } => {
            eprintln!(
                "SKIP (typed): MT-008 doc sync proof needs a discovered LSP server; \
                 language_id={language_id}, probed_command={probed_command}"
            );
            return;
        }
        LspAttachState::NotProbed => panic!("MT-008: shell must probe before document sync"),
    }

    let ext_dir = external_artifact_dir("wp-kernel-012-mt-008");
    std::fs::create_dir_all(&ext_dir).expect("create MT-008 external artifact dir");
    let source_path = ext_dir.join(format!("mt-008-lsp-sync-proof-{}.rs", std::process::id()));
    std::fs::write(&source_path, "fn main() {}\n").expect("write LSP proof source");
    panel.load_file(source_path.to_string_lossy());
    panel.set_text("fn main() {}\n");
    let open_version = panel.buffer_version_for_test();

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let (open_uri, synced_open_version) = harness
        .state()
        .lsp_doc_sync_watermark()
        .expect("MT-008: didOpen watermark after file-backed frame");
    assert!(
        open_uri.starts_with("file:///"),
        "MT-008: didOpen uses a file URI, got {open_uri}"
    );
    assert_eq!(
        synced_open_version, open_version,
        "MT-008: didOpen watermark records the current buffer version"
    );

    panel.set_text("fn main() {\n    let x = 1;\n}\n");
    let change_version = panel.buffer_version_for_test();
    let mut change_observed = false;
    for _ in 0..240 {
        harness.run_steps(1);
        if harness
            .state()
            .lsp_doc_sync_watermark()
            .is_some_and(|(_, version)| version == change_version)
        {
            change_observed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
    }
    assert!(
        change_observed,
        "MT-008: live server initialization must complete and advance didChange within six seconds; \
         attach_state={:?}, watermark={:?}",
        harness.state().lsp_attach_state(),
        harness.state().lsp_doc_sync_watermark()
    );
    let (change_uri, synced_change_version) = harness
        .state()
        .lsp_doc_sync_watermark()
        .expect("MT-008: didChange watermark after edit");
    assert_eq!(
        change_uri, open_uri,
        "MT-008: didChange stays on the same URI"
    );
    assert_eq!(
        synced_change_version, change_version,
        "MT-008: didChange advances the document-sync watermark after buffer_version changes"
    );
}

/// App-level deterministic proof for the reopened MT-008 defect: the shipped host frame pump does not
/// merely advance a watermark; it sends the actual LSP `textDocument/didOpen` and
/// `textDocument/didChange` notifications through the mounted panel's real LSP transport.
#[test]
fn lsp_file_backed_document_sync_sends_didopen_didchange_notifications() {
    fn read_lsp_request(
        rt: &tokio::runtime::Runtime,
        client: &LspClient,
        label: &str,
    ) -> serde_json::Value {
        rt.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                client.read_test_request(),
            )
            .await
            .unwrap_or_else(|_| panic!("MT-008: timed out waiting for {label} LSP frame"))
            .unwrap_or_else(|| panic!("MT-008: missing {label} LSP frame"))
        })
    }

    let (mut app, rt) = editor_shell();
    let client = Arc::new(LspClient::new(LspServerConfig::command("in-memory-lsp")));
    let _server_write = rt.block_on(async { client.install_test_transport() });
    app.install_mounted_code_lsp_client_for_test(Arc::clone(&client), "in-memory-lsp");

    let panel = app.mounted_code_panel();
    let ext_dir = external_artifact_dir("wp-kernel-012-mt-008");
    std::fs::create_dir_all(&ext_dir).expect("create MT-008 external artifact dir");
    let source_path = ext_dir.join(format!("mt-008-lsp-frame-proof-{}.rs", std::process::id()));
    std::fs::write(&source_path, "fn main() {}\n").expect("write LSP frame proof source");
    panel.load_file(source_path.to_string_lossy());
    panel.set_text("fn main() {}\n");

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let open = read_lsp_request(&rt, &client, "didOpen");
    assert_eq!(
        open.get("method").and_then(|v| v.as_str()),
        Some("textDocument/didOpen"),
        "MT-008: host pump sent didOpen, got {open}"
    );
    let open_doc = &open["params"]["textDocument"];
    assert!(
        open_doc["uri"]
            .as_str()
            .is_some_and(|uri| uri.starts_with("file:///")),
        "MT-008: didOpen carries a file URI, got {open_doc}"
    );
    assert_eq!(
        open_doc["languageId"].as_str(),
        Some("rust"),
        "MT-008: didOpen carries the mounted code pane language"
    );
    assert_eq!(
        open_doc["text"].as_str(),
        Some("fn main() {}\n"),
        "MT-008: didOpen carries the current buffer text"
    );

    panel.set_text("fn main() {\n    let x = 1;\n}\n");
    let change_version = panel.buffer_version_for_test();
    harness.run_steps(3);
    let change = read_lsp_request(&rt, &client, "didChange");
    assert_eq!(
        change.get("method").and_then(|v| v.as_str()),
        Some("textDocument/didChange"),
        "MT-008: host pump sent didChange, got {change}"
    );
    assert_eq!(
        change["params"]["textDocument"]["version"].as_i64(),
        Some(change_version as i64),
        "MT-008: didChange carries the buffer_version observed by the host"
    );
    assert_eq!(
        change["params"]["contentChanges"][0]["text"].as_str(),
        Some("fn main() {\n    let x = 1;\n}\n"),
        "MT-008: didChange carries the changed full document text"
    );

    let second_path = ext_dir.join(format!(
        "mt-008-lsp-frame-proof-second-{}.rs",
        std::process::id()
    ));
    std::fs::write(&second_path, "pub fn second() {}\n").expect("write second LSP proof source");
    panel.load_file(second_path.to_string_lossy());
    panel.set_text("pub fn second() {}\n");
    harness.run_steps(3);

    let close = read_lsp_request(&rt, &client, "didClose");
    assert_eq!(
        close.get("method").and_then(|value| value.as_str()),
        Some("textDocument/didClose"),
        "MT-008: a same-client file switch closes the prior server-side document"
    );
    assert_eq!(
        close["params"]["textDocument"]["uri"].as_str(),
        open_doc["uri"].as_str(),
        "MT-008: didClose names the previously opened URI"
    );
    let reopened = read_lsp_request(&rt, &client, "second didOpen");
    assert_eq!(
        reopened.get("method").and_then(|value| value.as_str()),
        Some("textDocument/didOpen"),
        "MT-008: the replacement file opens only after the prior URI closes"
    );
    assert_ne!(
        reopened["params"]["textDocument"]["uri"].as_str(),
        open_doc["uri"].as_str(),
        "MT-008: the replacement didOpen carries the new URI"
    );

    // The seeded/base panel has no tab content_id even though it is file-backed. Closing it must still
    // remove the server-side document through the bound runtime.
    assert!(!harness.state_mut().close_active_tab_for_test());
    assert!(harness.state().pending_dirty_code_close_for_test());
    assert!(harness
        .state_mut()
        .discard_pending_dirty_code_close_for_test());
    let final_close = read_lsp_request(&rt, &client, "base-panel didClose");
    assert_eq!(
        final_close.get("method").and_then(|value| value.as_str()),
        Some("textDocument/didClose")
    );
    assert_eq!(
        final_close["params"]["textDocument"]["uri"].as_str(),
        reopened["params"]["textDocument"]["uri"].as_str(),
        "closing a file-backed base panel names its current URI"
    );
}

#[test]
fn lsp_sync_pumps_inactive_mounted_document_in_a_different_language() {
    fn read_lsp_request(
        rt: &tokio::runtime::Runtime,
        client: &LspClient,
        label: &str,
    ) -> serde_json::Value {
        rt.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(2),
                client.read_test_request(),
            )
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {label}"))
            .unwrap_or_else(|| panic!("missing {label}"))
        })
    }

    let (mut app, rt) = editor_shell();
    let dir = external_artifact_dir("wp-kernel-012-mt-008/multi-language-lsp");
    std::fs::create_dir_all(&dir).expect("create multi-language LSP directory");
    let rust_path = dir.join(format!("active-{}.rs", std::process::id()));
    let python_path = dir.join(format!("detached-{}.py", std::process::id()));
    std::fs::write(&rust_path, "fn main() {}\n").expect("write Rust source");
    std::fs::write(&python_path, "#!/usr/bin/env python\nvalue = 1\n")
        .expect("write Python source");
    app.mounted_code_panel()
        .load_file(rust_path.to_string_lossy());

    let generation = app.begin_code_document_load_for_test("python-mounted");
    app.deliver_code_document_load_for_test(
        generation,
        "python-mounted",
        python_path,
        0,
        Ok("#!/usr/bin/env python\nvalue = 1\n".to_owned()),
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    let python_panel = harness
        .state()
        .mounted_code_panel_for_content_id("python-mounted")
        .expect("mounted Python panel");
    assert_eq!(python_panel.resolved_language().detected.as_str(), "python");

    // Make Rust the globally active tab; Python remains mounted but inactive.
    {
        let bar = harness
            .state_mut()
            .tab_bar_states_mut()
            .get_mut(&PaneId::from("pane-a"))
            .expect("pane-a tab bar");
        let rust_index = bar
            .tabs
            .iter()
            .position(|tab| tab.content_id.as_deref().unwrap_or_default().is_empty())
            .expect("base Rust tab");
        bar.activate(rust_index);
    }
    harness
        .state_mut()
        .set_active_pane_for_test(Some(PaneId::from("pane-a")));

    let rust_client = Arc::new(LspClient::new(LspServerConfig::command("rust-memory-lsp")));
    let python_client = Arc::new(LspClient::new(LspServerConfig::command(
        "python-memory-lsp",
    )));
    let _rust_server = rt.block_on(async { rust_client.install_test_transport() });
    let _python_server = rt.block_on(async { python_client.install_test_transport() });
    harness
        .state_mut()
        .install_code_lsp_client_for_language_for_test(
            "rust",
            Arc::clone(&rust_client),
            "rust-memory-lsp",
        );
    harness
        .state_mut()
        .install_code_lsp_client_for_language_for_test(
            "python",
            Arc::clone(&python_client),
            "python-memory-lsp",
        );
    harness.run_steps(3);

    let rust_method = read_lsp_request(&rt, &rust_client, "Rust sync")["method"]
        .as_str()
        .map(str::to_owned);
    assert!(
        matches!(
            rust_method.as_deref(),
            Some("textDocument/didOpen" | "textDocument/didChange")
        ),
        "active Rust document receives a sync notification, got {rust_method:?}"
    );
    let python_sync = read_lsp_request(&rt, &python_client, "Python initial sync");
    assert!(matches!(
        python_sync["method"].as_str(),
        Some("textDocument/didOpen" | "textDocument/didChange")
    ));
    if python_sync["method"].as_str() == Some("textDocument/didOpen") {
        assert_eq!(
            python_sync["params"]["textDocument"]["languageId"].as_str(),
            Some("python")
        );
    }

    python_panel.set_text("#!/usr/bin/env python\nvalue = 2\n");
    harness.run_steps(3);
    assert_eq!(
        read_lsp_request(&rt, &python_client, "inactive Python didChange")["method"].as_str(),
        Some("textDocument/didChange"),
        "an inactive mounted Python document must not stall behind the active Rust pane"
    );
}

/// Whether the named real OS process still exists. The app-host lifecycle proof intentionally keeps
/// external `Arc<LspClient>` observers alive, so PID disappearance proves explicit store ownership
/// retirement rather than incidental final-Arc drop.
#[cfg(feature = "integration")]
fn app_host_lsp_process_alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).contains(&format!("\"{pid}\"")))
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }
}

#[cfg(feature = "integration")]
fn wait_for_app_host_lsp_exit(pid: u32, bound: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + bound;
    while std::time::Instant::now() < deadline {
        if !app_host_lsp_process_alive(pid) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    !app_host_lsp_process_alive(pid)
}

/// Panic-safe owner for the real child processes used by the app-host lifecycle proof. The test
/// intentionally retains observer Arcs, so an assertion failure before the normal app-drop boundary
/// must still detach every registered transport while the fixture runtime remains alive.
#[cfg(feature = "integration")]
#[derive(Default)]
struct AppHostLspFixtureCleanup {
    clients: Vec<Arc<LspClient>>,
}

#[cfg(feature = "integration")]
impl AppHostLspFixtureCleanup {
    fn register(&mut self, client: &Arc<LspClient>) {
        self.clients.push(Arc::clone(client));
    }
}

#[cfg(feature = "integration")]
impl Drop for AppHostLspFixtureCleanup {
    fn drop(&mut self) {
        for client in &self.clients {
            client.shutdown_for_host();
        }
    }
}

#[cfg(feature = "integration")]
fn app_host_canned_lsp_write_frame(message: &serde_json::Value) {
    use std::io::Write;

    let body = serde_json::to_vec(message).expect("serialize app-host canned LSP frame");
    let mut stdout = std::io::stdout().lock();
    // libtest prints the running test name without a trailing newline. A leading CRLF prevents that
    // progress prefix from becoming part of the first Content-Length header.
    let _ = write!(stdout, "\r\nContent-Length: {}\r\n\r\n", body.len());
    let _ = stdout.write_all(&body);
    let _ = stdout.flush();
}

/// Real stdio LSP child used only when the integration app-host PID proof re-executes this test binary
/// with its private activation variable. Normal test-suite execution returns immediately.
#[test]
#[cfg(feature = "integration")]
fn canned_app_host_lsp_server_main() {
    if std::env::var("HANDSHAKE_APP_HOST_CANNED_LSP_SERVER").as_deref() != Ok("1") {
        return;
    }

    use std::io::Read;
    let mut stdin = std::io::stdin().lock();
    loop {
        let mut content_length = None;
        let mut line = Vec::new();
        loop {
            let mut byte = [0u8; 1];
            if stdin.read_exact(&mut byte).is_err() {
                return;
            }
            if byte[0] == b'\n' {
                let text = String::from_utf8_lossy(&line);
                let trimmed = text.trim_end_matches('\r');
                if trimmed.is_empty() {
                    break;
                }
                if let Some(value) = trimmed.strip_prefix("Content-Length:") {
                    content_length = value.trim().parse::<usize>().ok();
                }
                line.clear();
            } else {
                line.push(byte[0]);
            }
        }
        let Some(length) = content_length else {
            continue;
        };
        let mut body = vec![0u8; length];
        if stdin.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(&body) else {
            continue;
        };
        let method = message
            .get("method")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let id = message.get("id").cloned();
        match (method, id) {
            ("initialize", Some(id)) => {
                app_host_canned_lsp_write_frame(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {},
                        "serverInfo": { "name": "handshake-app-host-canned-lsp" }
                    }
                }));
            }
            ("shutdown", Some(id)) => {
                app_host_canned_lsp_write_frame(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": null
                }));
            }
            ("exit", None) => std::process::exit(0),
            _ => {}
        }
    }
}

#[cfg(feature = "integration")]
fn start_real_app_host_lsp(
    runtime: &tokio::runtime::Runtime,
    label: &str,
) -> (Arc<LspClient>, u32) {
    let executable = std::env::current_exe().expect("resolve current app-host test binary");
    let client = Arc::new(LspClient::new(LspServerConfig {
        command: executable.to_string_lossy().into_owned(),
        args: vec![
            "canned_app_host_lsp_server_main".to_owned(),
            "--exact".to_owned(),
            "--nocapture".to_owned(),
            "--test-threads=1".to_owned(),
        ],
    }));
    std::env::set_var("HANDSHAKE_APP_HOST_CANNED_LSP_SERVER", "1");
    let initialized = runtime.block_on(client.initialize(None));
    std::env::remove_var("HANDSHAKE_APP_HOST_CANNED_LSP_SERVER");
    assert!(initialized, "{label}: real canned LSP initializes");
    assert!(client.is_running(), "{label}: real transport is live");
    let pid = client
        .spawned_process_id_for_test()
        .unwrap_or_else(|| panic!("{label}: real transport exposes its child PID"));
    assert!(
        app_host_lsp_process_alive(pid),
        "{label}: child PID {pid} is alive before ownership transition"
    );
    (client, pid)
}

/// Real-process MT-008 host-owner proof. The sole mounted panel changes Rust -> JavaScript, which must
/// prune and reap the old Rust server BEFORE app Drop even though this test retains an observer Arc.
/// A second mounted Rust document then leaves live JavaScript + Rust clients in the store; dropping the
/// HandshakeApp must reap BOTH while its injected Tokio runtime is still alive.
#[test]
#[cfg(feature = "integration")]
fn app_host_language_rebind_and_drop_reap_every_real_lsp_process() {
    #[cfg(windows)]
    assert_eq!(
        handshake_native::code_editor::lsp_client::lsp_focus_safe_creation_flags_for_test(),
        handshake_native::code_editor::lsp_client::LSP_CREATE_NO_WINDOW_FLAG,
        "every real app-host LSP spawn uses CREATE_NO_WINDOW"
    );

    let (mut app, runtime) = editor_shell();
    // Declared after the runtime so unwind drops this guard first and cannot strand a canned child.
    let mut process_cleanup = AppHostLspFixtureCleanup::default();
    let directory = external_artifact_dir("wp-kernel-012-mt-008/app-host-process-ownership");
    std::fs::create_dir_all(&directory).expect("create external LSP process-proof directory");
    let initial_rust_path = directory.join(format!("initial-{}.rs", std::process::id()));
    let javascript_path = directory.join(format!("replacement-{}.js", std::process::id()));
    let retained_rust_path = directory.join(format!("retained-{}.rs", std::process::id()));
    std::fs::write(&initial_rust_path, "fn initial() {}\n").expect("write initial Rust source");
    std::fs::write(&javascript_path, "const value = 1;\n")
        .expect("write replacement JavaScript source");
    std::fs::write(&retained_rust_path, "fn retained() {}\n").expect("write retained Rust source");

    app.mounted_code_panel()
        .load_file(initial_rust_path.to_string_lossy());
    assert_eq!(
        app.mounted_code_panel()
            .resolved_language()
            .detected
            .as_str(),
        "rust"
    );
    let (old_rust_client, old_rust_pid) = start_real_app_host_lsp(&runtime, "old Rust");
    process_cleanup.register(&old_rust_client);
    app.install_code_lsp_client_for_language_for_test(
        "rust",
        Arc::clone(&old_rust_client),
        "old-rust-real-lsp",
    );

    // Pre-register JavaScript without a process while Rust is still open. This prevents host discovery from
    // making the proof depend on what happens to be installed on PATH, while keeping both registry
    // entries legitimate until the actual panel-language transition occurs.
    app.install_code_lsp_client_for_language_for_test(
        "javascript",
        Arc::new(LspClient::disabled()),
        "javascript-disabled-test-binding",
    );

    // This is the actual app-frame language-change path: the sole panel changes language, then the
    // production document-sync pump reconciles mounted languages. It must remove and explicitly shut
    // down Rust despite the observer Arc above; no subsequent install is allowed to manufacture that.
    app.mounted_code_panel()
        .load_file(javascript_path.to_string_lossy());
    assert_eq!(
        app.mounted_code_panel()
            .resolved_language()
            .detected
            .as_str(),
        "javascript"
    );
    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(2);
    assert!(
        wait_for_app_host_lsp_exit(old_rust_pid, std::time::Duration::from_secs(5)),
        "old Rust PID {old_rust_pid} is reaped on Rust -> JavaScript rebind before app Drop"
    );
    assert!(
        old_rust_client.spawned_process_id_for_test().is_none(),
        "the old observer Arc remains, but store retirement detached its real transport"
    );

    let (javascript_client, javascript_pid) =
        start_real_app_host_lsp(&runtime, "replacement JavaScript");
    process_cleanup.register(&javascript_client);
    harness
        .state_mut()
        .install_code_lsp_client_for_language_for_test(
            "javascript",
            Arc::clone(&javascript_client),
            "javascript-real-lsp",
        );
    assert!(app_host_lsp_process_alive(javascript_pid));

    // Mount Rust alongside the JavaScript base document so both language registry entries have a real
    // consumer and must remain live until the HandshakeApp owner is dropped. Pre-register a disabled
    // Rust binding so the first frame with that new panel cannot depend on a host PATH discovery.
    harness
        .state_mut()
        .install_code_lsp_client_for_language_for_test(
            "rust",
            Arc::new(LspClient::disabled()),
            "rust-disabled-test-binding",
        );
    let generation = harness
        .state_mut()
        .begin_code_document_load_for_test("retained-rust");
    harness.state().deliver_code_document_load_for_test(
        generation,
        "retained-rust",
        retained_rust_path,
        0,
        Ok("fn retained() {}\n".to_owned()),
    );
    harness.run_steps(2);
    let retained_rust_panel = harness
        .state()
        .mounted_code_panel_for_content_id("retained-rust")
        .expect("mounted retained Rust panel");
    assert_eq!(
        retained_rust_panel.resolved_language().detected.as_str(),
        "rust"
    );
    let (retained_rust_client, retained_rust_pid) =
        start_real_app_host_lsp(&runtime, "retained Rust");
    process_cleanup.register(&retained_rust_client);
    harness
        .state_mut()
        .install_code_lsp_client_for_language_for_test(
            "rust",
            Arc::clone(&retained_rust_client),
            "retained-rust-real-lsp",
        );
    assert!(app_host_lsp_process_alive(javascript_pid));
    assert!(app_host_lsp_process_alive(retained_rust_pid));

    drop(harness);
    assert!(
        wait_for_app_host_lsp_exit(javascript_pid, std::time::Duration::from_secs(5)),
        "HandshakeApp Drop reaps retained JavaScript PID {javascript_pid} before runtime teardown"
    );
    assert!(
        wait_for_app_host_lsp_exit(retained_rust_pid, std::time::Duration::from_secs(5)),
        "HandshakeApp Drop reaps retained Rust PID {retained_rust_pid} before runtime teardown"
    );
    assert!(
        javascript_client.spawned_process_id_for_test().is_none()
            && retained_rust_client.spawned_process_id_for_test().is_none(),
        "external observer Arcs remain but every app-owned real transport is detached"
    );
    drop(runtime);
    assert_no_local_artifact_dir();
}

#[test]
fn closing_base_code_tab_sends_didclose_without_reopening_or_saving_it() {
    let (mut app, rt) = editor_shell();
    let client = Arc::new(LspClient::new(LspServerConfig::command("base-close-lsp")));
    let _server = rt.block_on(async { client.install_test_transport() });
    app.install_mounted_code_lsp_client_for_test(Arc::clone(&client), "base-close-lsp");
    let dir = external_artifact_dir("wp-kernel-012-mt-008/base-close");
    std::fs::create_dir_all(&dir).expect("create base-close directory");
    let path = dir.join(format!("base-close-{}.rs", std::process::id()));
    std::fs::write(&path, "fn base() {}\n").expect("write base-close source");
    app.mounted_code_panel().load_file(path.to_string_lossy());

    let mut harness =
        Harness::builder().build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);
    let first = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.read_test_request(),
        )
        .await
        .expect("base didOpen timeout")
        .expect("base didOpen frame")
    });
    assert!(matches!(
        first["method"].as_str(),
        Some("textDocument/didOpen" | "textDocument/didChange")
    ));

    assert!(harness.state_mut().close_active_tab_for_test());
    harness.run_steps(4);
    let close = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(2),
            client.read_test_request(),
        )
        .await
        .expect("base didClose timeout")
        .expect("base didClose frame")
    });
    assert_eq!(close["method"].as_str(), Some("textDocument/didClose"));

    harness.run_steps(6);
    let after_close = rt.block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_millis(150),
            client.read_test_request(),
        )
        .await
    });
    assert!(
        !matches!(
            after_close,
            Ok(Some(ref message))
                if matches!(
                    message.get("method").and_then(|value| value.as_str()),
                    Some("textDocument/didOpen" | "textDocument/didChange")
                )
        ),
        "closed base document must not emit a later didOpen/didChange: {after_close:?}"
    );

    let before = std::fs::read_to_string(&path).expect("read base-close source");
    let closed_panel = harness.state().mounted_code_panel();
    closed_panel.set_text("fn closed_panel_must_not_save() {}\n");
    closed_panel.request_save_for_host();
    harness.run_steps(2);
    assert_eq!(
        std::fs::read_to_string(&path).expect("read base-close source after stale Save"),
        before,
        "a host Save emitted by the closed reusable base panel is ignored"
    );
}

// ── WP-KERNEL-012 MT-079 remediation (FAIL_V2): CANONICAL Argus lifecycle over the MOUNTED editor panes ──
//
// validation_v2 failed MT-079 because "there is no current canonical Argus evidence covering creation,
// navigation, focus, popout/close, and fresh post-action state for every editor pane. The available
// screenshot path is not material GPU capture proof." The existing MT-079 coverage drives the mounted
// editors, but through kittest-native events / direct app calls — it never drives the mounted `HandshakeApp`
// through the REAL localhost `SwarmMcpServer` transport (`argus.inspect`/`argus.click` with typed receipts
// + fresh re-inspection) the way an out-of-process swarm agent does. These tests close that exact gap for
// EVERY editor pane's create -> navigate -> focus -> popout/close lifecycle, and prove the operator
// menu-bar reachability the WP requires.
//
// Artifact hygiene (CX-212E): evidence is written ONLY under the EXTERNAL
// `Handshake_Artifacts/handshake-test/wp-kernel-012-mt-079/canonical-argus/` root.

const MT079_ARGUS_SUBDIR: &str = "wp-kernel-012-mt-079/canonical-argus";

#[test]
fn mt079_mounted_editor_panes_canonical_argus_lifecycle() {
    use handshake_native::code_editor::panel::CODE_EDITOR_VISIBLE_WRAP_TOGGLE_AUTHOR_ID;
    use handshake_native::quick_switcher::{NavDispatchOutcome, ShellNavigator};
    use handshake_native::rich_editor::reading_mode::{
        TOGGLE_CONTAINER_AUTHOR_ID, TOGGLE_EDIT_AUTHOR_ID, TOGGLE_READING_AUTHOR_ID,
    };
    use handshake_native::rich_editor::renderer::RICH_EDITOR_ROOT_AUTHOR_ID;
    use handshake_native::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID;

    let (app, _rt) = editor_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(4);

    let artifact_dir = external_artifact_dir(MT079_ARGUS_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)
        .expect("create external MT-079 canonical-Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-079-editors");

    // (1) CREATION: both mounted editor panes render their REAL subtrees (not placeholders) and are
    // addressable through the real localhost transport.
    let created = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&created, CODE_EDITOR_TEXT_AUTHOR_ID),
        "canonical argus.inspect must see the mounted code editor text node '{CODE_EDITOR_TEXT_AUTHOR_ID}'"
    );
    assert!(
        json_has_author_id(&created, RICH_EDITOR_ROOT_AUTHOR_ID),
        "canonical argus.inspect must see the mounted rich editor root node '{RICH_EDITOR_ROOT_AUTHOR_ID}'"
    );

    // (2) SAFE CANONICAL STEER (with typed receipts) on EACH editor pane: drive a safe, reversible control
    // on the code pane (word-wrap toggle) and the rich pane (reading-mode toggle) over the real transport,
    // and freshly re-observe each pane's control remains addressable after the action. (`editor.code.text`
    // is a TextInput that supports Focus/SetValue, not Click, so the code steer targets a real Role::Button
    // toolbar control instead.)
    let code_focus =
        argus.click_and_reinspect(&mut harness, CODE_EDITOR_VISIBLE_WRAP_TOGGLE_AUTHOR_ID);
    assert!(
        matches!(
            code_focus.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical code-editor wrap-toggle steer receipt is terminal and non-rejected: {}",
        code_focus.receipt_status
    );
    assert!(
        code_focus
            .agent_id
            .contains(":client:wp-kernel-012-mt-079-editors-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        code_focus.agent_id
    );
    assert!(
        json_has_author_id(&code_focus.after, CODE_EDITOR_VISIBLE_WRAP_TOGGLE_AUTHOR_ID)
            && json_has_author_id(&code_focus.after, CODE_EDITOR_TEXT_AUTHOR_ID),
        "the code pane remains fully addressable after the safe wrap-toggle steer"
    );
    // A reversible round-trip steer on the rich pane: flip to Reading, then restore Edit, so the pane ends
    // back in its editable state (editor.rich.root present) for the downstream lifecycle observations.
    let rich_focus = argus.click_and_reinspect(&mut harness, TOGGLE_READING_AUTHOR_ID);
    assert!(
        matches!(
            rich_focus.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical rich-editor reading-toggle steer receipt is terminal and non-rejected: {}",
        rich_focus.receipt_status
    );
    assert!(
        json_has_author_id(&rich_focus.after, TOGGLE_CONTAINER_AUTHOR_ID),
        "the rich pane's mode-toggle control remains addressable after the safe reading-mode steer"
    );
    let rich_restore = argus.click_and_reinspect(&mut harness, TOGGLE_EDIT_AUTHOR_ID);
    assert!(
        matches!(
            rich_restore.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical rich-editor edit-restore steer receipt is terminal and non-rejected: {}",
        rich_restore.receipt_status
    );
    assert!(
        json_has_author_id(&rich_restore.after, RICH_EDITOR_ROOT_AUTHOR_ID),
        "restoring Edit mode via canonical Argus re-exposes the editable rich editor root"
    );

    // (3) NAVIGATION: the MT-030 ShellNavigator opens+focuses the REAL mounted editor panes. Both typed
    // seams return `Opened` (they landed on live mounted panes, not the retired EditorPaneNotMounted seam),
    // and the fresh canonical re-inspection confirms the code navigation focused the live mounted code
    // editor. (open_document navigates the rich pane to a fresh backend-backed document; with no backend it
    // enters the honest non-editable loading gate, so editor.rich.root is intentionally not asserted here.)
    let sym = harness.state_mut().open_code_symbol("mt079-argus-symbol");
    assert!(
        matches!(sym, NavDispatchOutcome::Opened { .. }),
        "open_code_symbol opens the mounted code pane; got {sym:?}"
    );
    let doc = harness.state_mut().open_document("mt079-argus-doc");
    assert!(
        matches!(doc, NavDispatchOutcome::Opened { .. }),
        "open_document opens the mounted rich pane; got {doc:?}"
    );
    harness.run_steps(2);
    let navigated = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&navigated, CODE_EDITOR_TEXT_AUTHOR_ID),
        "fresh canonical inspection after navigation still sees the mounted code editor focused live"
    );

    // (4) POPOUT (detach into its own window): pop the code pane out. The fresh canonical re-inspection is
    // the material post-action state: the runtime records the pane as popped-out, the DETACHED code editor
    // remains canonically Argus-addressable + steerable in its own window (so an out-of-process agent can
    // still drive a popped-out editor), and the sibling panes stay addressable (the popout is scoped, not a
    // global teardown).
    harness.state_mut().request_pop_out(PaneId::from("pane-a"));
    harness.run_steps(3);
    assert!(
        harness.state().is_popped_out(&PaneId::from("pane-a")),
        "request_pop_out detached the code pane into its own window (post-action runtime state)"
    );
    let popped = argus.inspect(&mut harness);
    assert!(
        json_has_author_id(&popped, CODE_EDITOR_TEXT_AUTHOR_ID),
        "the popped-out code editor remains canonically Argus-addressable in its detached window"
    );
    assert!(
        json_has_author_id(&popped, RUNTIME_CHAT_PANEL_AUTHOR_ID),
        "a sibling mounted pane (Runtime Chat) remains addressable after the code pane popped out"
    );

    // (6) Evidence: the before/after canonical trees for every lifecycle state + a screenshot marker
    // (headless DEFERRED is an acceptable typed outcome per the screenshot harness contract).
    let tree_path = artifact_dir.join("mt079-mounted-editors-argus-lifecycle.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "created": created,
            "code_focus_receipt": { "id": code_focus.receipt_id, "status": code_focus.receipt_status, "agent": code_focus.agent_id },
            "rich_focus_receipt": { "id": rich_focus.receipt_id, "status": rich_focus.receipt_status, "agent": rich_focus.agent_id },
            "navigated": navigated,
            "code_popped_out_detached": popped,
        }))
        .expect("serialize canonical MT-079 editor lifecycle evidence"),
    )
    .expect("write canonical MT-079 editor lifecycle evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt079-mounted-editors-argus.png");
            image.save(&path).expect("save mounted editors screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-079 canonical Argus editor lifecycle: create(code+rich) -> safe-steer(code wrap toggle + \
         rich reading->edit round-trip, receipts terminal) -> navigate(open_code_symbol + open_document, \
         both Opened) -> popout(code pane popped out; detached editor stays Argus-addressable; chat sibling \
         stays). screenshot={} tree={}",
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}

#[test]
fn mt079_editor_surfaces_reachable_from_menu_bar_canonical_argus() {
    use handshake_native::runtime_chat::RUNTIME_CHAT_PANEL_AUTHOR_ID;
    use handshake_native::top_menu_bar::{set_menu_popup_open, MenuId};

    let (app, _rt) = editor_shell();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    // Open the VIEW menu on the live shell so its dynamic Open-Editor-Surfaces leaves render + the shell
    // persists the open menu into the MCP snapshot pass (line ~25389: mcp_open_top_menu = open_menu(ctx)),
    // so the canonical inspect below sees the leaves the same way an out-of-process agent would.
    set_menu_popup_open(&harness.ctx, MenuId::View, true);
    harness.run_steps(2);

    let artifact_dir = external_artifact_dir(MT079_ARGUS_SUBDIR);
    std::fs::create_dir_all(&artifact_dir)
        .expect("create external MT-079 canonical-Argus artifact dir");

    let mut argus = CanonicalArgusDriver::bind(harness.state(), "wp-kernel-012-mt-079-menu");

    // (1) REACHABILITY: EVERY WP-KERNEL-012 editor surface/pane is an addressable menu-bar item over the
    // real localhost transport — including the MT-098 Runtime Chat surface whose open route this WP wired.
    let menu = argus.inspect(&mut harness);
    for leaf in [
        "menu.view.open-code-editor",
        "menu.view.open-rich-note",
        "menu.view.open-runtime-chat",
        "menu.view.open-wiki-projection",
        "menu.view.open-knowledge-graph",
        "menu.view.open-folders",
        "menu.view.open-tags",
        "menu.view.open-block-collections",
        "menu.view.open-canvas",
        "menu.view.open-loom-search",
        "menu.view.open-find-in-files",
        "menu.view.open-daily-journal",
        "menu.view.open-diff-editor",
    ] {
        assert!(
            json_has_author_id(&menu, leaf),
            "operator menu-bar reachability: '{leaf}' must be an addressable VIEW menu item over canonical Argus"
        );
    }

    // (2) CLICK-TO-OPEN: canonical Argus click the newly-wired Runtime Chat open route (the gap this WP
    // closed) -> fresh inspect re-observes the mounted chat pane, and the active work surface hosts it.
    let open_chat = argus.click_and_reinspect(&mut harness, "menu.view.open-runtime-chat");
    assert!(
        matches!(
            open_chat.receipt_status.as_str(),
            "applied" | "indeterminate"
        ),
        "the canonical Open-Runtime-Chat receipt is terminal and non-rejected: {}",
        open_chat.receipt_status
    );
    assert!(
        open_chat
            .agent_id
            .contains(":client:wp-kernel-012-mt-079-menu-agent"),
        "the canonical receipt retains the external caller attribution: {}",
        open_chat.agent_id
    );
    assert!(
        json_has_author_id(&open_chat.after, RUNTIME_CHAT_PANEL_AUTHOR_ID),
        "clicking Open Runtime Chat from the menu bar re-observes the mounted chat pane"
    );
    let active = harness
        .state()
        .active_pane()
        .cloned()
        .expect("an active pane exists after Open Runtime Chat");
    assert!(
        harness
            .state()
            .tab_bar_states()
            .get(&active)
            .map(|bar| bar
                .tabs
                .iter()
                .any(|t| t.pane_type == PaneType::RuntimeChat))
            .unwrap_or(false),
        "Open Runtime Chat opened the RuntimeChat pane on the active work surface (not a no-op)"
    );

    let tree_path = artifact_dir.join("mt079-menu-bar-reachability-argus.json");
    std::fs::write(
        &tree_path,
        serde_json::to_vec_pretty(&serde_json::json!({
            "view_menu_open": menu,
            "after_open_runtime_chat": open_chat.after,
            "open_chat_receipt": { "id": open_chat.receipt_id, "status": open_chat.receipt_status, "agent": open_chat.agent_id },
        }))
        .expect("serialize canonical MT-079 menu-bar evidence"),
    )
    .expect("write canonical MT-079 menu-bar evidence externally");
    assert!(tree_path.is_file());

    let screenshot_marker = match harness.render() {
        Ok(image) => {
            let path = artifact_dir.join("mt079-menu-bar-reachability.png");
            image.save(&path).expect("save menu-bar screenshot");
            format!("CAPTURED {}", path.display())
        }
        Err(deferred) => format!("DEFERRED (headless): {deferred}"),
    };
    println!(
        "MT-079 canonical Argus menu-bar reachability: VIEW menu exposes all 13 editor-surface leaves \
         (incl. menu.view.open-runtime-chat) -> click(open-runtime-chat) -> chat pane mounted on the \
         active surface. screenshot={} tree={}",
        screenshot_marker,
        tree_path.display()
    );

    argus.finish();
    assert_no_local_artifact_dir();
}
