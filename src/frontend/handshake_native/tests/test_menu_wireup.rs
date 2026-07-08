//! WP-KERNEL-012 MT-069 (E11 Shell-menu wire-up) — the editor FILE/EDIT menu + command-palette items
//! WP-011 shipped as honestly-DISABLED placeholders now dispatch the REAL WP-012 editor commands through
//! the EXISTING shell single substrate (the MT-031 InteractionBus command ids + the MT-020 editor save
//! path + the MT-035 unified-undo scope), proven against the LIVE `HandshakeApp` tree.
//!
//! These tests drive the actual shell (the `editor_shell()` host-mount harness from the MT-079 pattern,
//! with the seeded panes re-typed to the mounted code + Notes editors), NOT a widget harness, so the menu
//! click → command-id dispatch → shared-substrate effect is proven end-to-end the SAME out-of-process way
//! a swarm agent drives it.
//!
//! - AC-001 / AC-002 / PT-002: each formerly-disabled FILE/EDIT editor menu item dispatches its real
//!   editor command via the command bus when clicked against the live tree (observed via the editor-state
//!   mutation or the recorded `last_editor_command` dispatch).
//! - AC-003 / PT-005: the GO-menu code-nav items render ENABLED for the active code target with typed
//!   logged no-ops when the active target is wrong (never a panic),
//!   and a static source scan proves zero `todo!()`/`unimplemented!()`/`panic!()` on the wired handler
//!   bodies in `top_menu_bar.rs` + `command_registry.rs` + `app.rs`.
//! - AC-004 / PT-003: Save routes through the MT-020 editor save path (NOT a shell-local save); Undo/Redo
//!   route through the MT-035 unified-undo scope so menu undo and the same stack the keyboard uses are ONE
//!   stack (a menu Undo pops the unified-scope entry).
//! - AC-005 / PT-004: the previously-disabled command-palette editor entries are now enabled and dispatch
//!   real handlers; the Quick Switcher entry is likewise enabled.
//! - AC-006 / PT-001: no menu/palette item dispatches a panic on the required path (the runtime drive of
//!   every item completes without panic).
//! - AC-007: the menu/palette handlers route by command id ONLY (no inline editor logic) — a source scan
//!   of the dispatch call sites.
//! - AC-008 / PT-006: the AccessKit tree dump shows a formerly-disabled item (`menu.file.save`) now an
//!   ENABLED MenuItem node carrying its WP-011 author_id.

use std::path::{Path, PathBuf};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::app::{HandshakeApp, HealthDisplayState, DEFAULT_PROJECT_ID};
use handshake_native::backend_client::HealthInfo;
use handshake_native::command_registry::{
    self, CommandKind, CMD_EDITOR_EDIT_REDO, CMD_EDITOR_EDIT_UNDO, CMD_EDITOR_FILE_SAVE,
    CMD_EDITOR_GO_TO_DEFINITION, CMD_VIEW_CANVAS, CMD_VIEW_CODE_EDITOR, CMD_VIEW_DIFF_MERGE,
    CMD_VIEW_FIND_IN_FILES, CMD_VIEW_GRAPH, CMD_VIEW_JOURNAL, CMD_VIEW_LOOM_SEARCH,
    CMD_VIEW_RICH_NOTE, CMD_WORKBENCH_QUICK_OPEN, EDITOR_GO_NAV_PENDING_IDS,
};
use handshake_native::interop::InteractionBus;
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneId, PaneRecord, PaneType,
};

/// Serialize the `.wgpu()` screenshot test (the documented Windows-wgpu concurrent-device hazard).
static WGPU_SERIAL_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());
fn wgpu_guard() -> std::sync::MutexGuard<'static, ()> {
    WGPU_SERIAL_GUARD.lock().unwrap_or_else(|p| p.into_inner())
}

/// The crate-relative path to the EXTERNAL artifacts root (CX-212E), disk-agnostic — mirrors the MT-079
/// host-mount test's helper. The SCREENSHOT/TEST-ARTIFACT rule overrides any repo-local path; artifacts go
/// here ONLY.
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

/// Artifact-hygiene guard the SCREENSHOT/TEST-ARTIFACT rule mandates: NO repo-local artifact directory may
/// exist under the crate (checks BOTH `test_output/` and `tests/screenshots/`).
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

/// A live, RUNTIME-INJECTED shell with the seeded 2x2 panes RE-TYPED so an editor pane is the focusable
/// target — the top-left slot hosts the code editor (`PaneType::CodeSymbol`) and the top-right the
/// Notes/rich editor (`PaneType::LoomWikiPage`), the two surfaces MT-079 mounts the real editor factories
/// over. With an editor pane present, the MT-069 enable predicate (`editor_available`) is TRUE, so the
/// FILE/EDIT editor menu + palette items render ENABLED. The runtime outlives the harness (a dropped
/// runtime would unbind the editors mid-test).
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

/// A deliberately non-editor shell — MT-097's fresh default now has code + Notes editors, so this test
/// fixture retypes those slots to non-editor panes to keep the disabled-when-unavailable proof honest.
fn plain_shell() -> HandshakeApp {
    let mut app = HandshakeApp::with_health(HealthDisplayState::Ok(HealthInfo {
        status: "ok".to_string(),
        db_status: "ok".to_string(),
        migration_version: Some(1),
    }));
    {
        let registry = app.pane_registry();
        let mut guard = registry.lock().expect("registry");
        guard.insert(PaneRecord::new(
            PaneId::from("pane-a"),
            PaneType::Workspace,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
        guard.insert(PaneRecord::new(
            PaneId::from("pane-b"),
            PaneType::InferenceLab,
            DEFAULT_PROJECT_ID,
            None,
            LockState::Unlocked,
            DirtyState::Clean,
            PaneAuthority::System,
        ));
    }
    {
        let bars = app.tab_bar_states_mut();
        if let Some(bar) = bars.get_mut(&PaneId::from("pane-a")) {
            bar.tabs = vec![handshake_native::tab_bar::TabState::new(
                PaneType::Workspace,
            )];
            bar.active_index = 0;
        }
        if let Some(bar) = bars.get_mut(&PaneId::from("pane-b")) {
            bar.tabs = vec![handshake_native::tab_bar::TabState::new(
                PaneType::InferenceLab,
            )];
            bar.active_index = 0;
        }
    }
    app
}

fn shell_harness(app: HandshakeApp) -> Harness<'static, HandshakeApp> {
    Harness::builder().build_state(|ctx, a: &mut HandshakeApp| a.ui(ctx), app)
}

/// Collect every live AccessKit node carrying an author_id: (author_id, role, is_disabled).
fn live_author_nodes(harness: &Harness<'_, HandshakeApp>) -> Vec<(String, String, bool)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((
                author_id.to_owned(),
                format!("{:?}", ak.role()),
                ak.is_disabled(),
            ));
        }
    }
    found
}

fn live_author_ids(harness: &Harness<'_, HandshakeApp>) -> Vec<String> {
    live_author_nodes(harness)
        .into_iter()
        .map(|(author_id, _, _)| author_id)
        .collect()
}

fn assert_active_tab_contains(
    harness: &Harness<'_, HandshakeApp>,
    pane_type: PaneType,
    context: &str,
) {
    let target = harness
        .state()
        .active_pane()
        .cloned()
        .unwrap_or_else(|| panic!("{context}: an active pane exists"));
    let opened = harness
        .state()
        .tab_bar_states()
        .get(&target)
        .map(|bar| bar.tabs.iter().any(|t| t.pane_type == pane_type))
        .unwrap_or(false);
    assert!(
        opened,
        "{context}: active pane tab list contains {pane_type:?}"
    );
}

fn assert_live_author_id(harness: &Harness<'_, HandshakeApp>, author_id: &str, context: &str) {
    let ids = live_author_ids(harness);
    assert!(
        ids.iter().any(|id| id == author_id),
        "{context}: expected rendered author_id '{author_id}', got {ids:?}"
    );
}

/// Resolve a stable `author_id` to its live AccessKit NodeId in the harness tree. Labels are ambiguous
/// in the live shell (e.g. several "Undo" nodes: the menu leaf, a toolbar button, a code-editor command
/// button), so the MENU leaves are addressed by their UNIQUE author_id — the SAME out-of-process address
/// a swarm agent uses (HBR-SWARM).
fn node_id_for(harness: &Harness<'_, HandshakeApp>, author_id: &str) -> egui::accesskit::NodeId {
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if ak.author_id() == Some(author_id) {
            return ak.id();
        }
    }
    panic!("author_id '{author_id}' not found in the live tree");
}

/// Click the live node addressed by `author_id` through an AccessKit `Action::Click` request — the EXACT
/// out-of-process dispatch path a swarm agent / the UIA adapter uses (not a label lookup). This is the
/// genuine "menu item clicked by a model" path the contract requires (HBR-SWARM / HBR-VIS).
fn click_author_id(harness: &mut Harness<'_, HandshakeApp>, author_id: &str) {
    let node_id = node_id_for(harness, author_id);
    harness.event(egui::Event::AccessKitActionRequest(
        egui::accesskit::ActionRequest {
            action: egui::accesskit::Action::Click,
            target: node_id,
            data: None,
        },
    ));
}

fn dispatch_palette_live(harness: &mut Harness<'_, HandshakeApp>, command_id: &str) -> bool {
    let ctx = harness.ctx.clone();
    harness
        .state_mut()
        .dispatch_palette_action_for_test_with_ctx(&ctx, command_id)
}

// ── AC-001 / AC-004 / PT-002 / PT-003: FILE > Save dispatches the MT-020 editor save path ──────────────

#[test]
fn file_save_dispatches_editor_save_path() {
    let (mut app, _rt) = editor_shell();
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_RICH_NOTE),
        "precondition: activate the mounted rich editor pane without binding a backend document id"
    );
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // No editor command observed yet, and the MT-020 SaveManager is NOT in flight before the menu Save.
    assert!(
        harness.state().last_editor_command().is_none(),
        "no editor command before the menu Save"
    );
    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "the MT-020 SaveManager is idle before the menu Save"
    );

    // Open FILE, click the now-ENABLED Save leaf by its UNIQUE author_id (the genuine out-of-process
    // swarm-agent click path — labels are ambiguous in the live shell).
    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    click_author_id(&mut harness, "menu.file.save");
    // One extra frame so the shell drains the code command channel (drive_editor_mounts) and records it.
    harness.run_steps(2);

    // AC-004 / PT-003 (the REAL MT-020 proof, NOT a host-set marker): the menu Save reached the MT-020
    // SaveManager save entry — `request_save` moved the SaveManager's OWN state machine into
    // `SaveState::Saving` (asserted via `save_is_in_flight`, a SaveManager-internal state the host does NOT
    // set). This proves the dispatch reaches the real MT-020 save path, not a shell-local/SQLite write.
    assert!(
        rich_state.lock().unwrap().save_is_in_flight(),
        "FILE > Save reached the MT-020 SaveManager save entry (request_save -> SaveState::Saving)"
    );
    // MT-069 REMEDIATION (Save SCOPING): with NO focused code pane, FILE > Save routes ONLY to the
    // rich SaveManager — the code pane's Save channel is NOT pinged (pre-remediation Save
    // unconditionally hit BOTH substrates, so a focused CODE pane's save silently saved the rich
    // document too). The scoped behavior is the fix this asserts.
    assert_eq!(
        harness.state().last_editor_command(),
        None,
        "MT-069: FILE > Save with no focused code pane must NOT ping the code Save channel \
         (save is scoped to the focused pane)"
    );
    // R6: the menu closed after the click.
    let nodes = live_author_nodes(&harness);
    assert!(
        !nodes.iter().any(|(a, _, _)| a == "menu.file.save"),
        "the FILE menu closed after Save was clicked: {nodes:?}"
    );
}

#[test]
fn file_save_scopes_to_active_code_editor_without_rich_save() {
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let rich_state = app.mounted_rich_state();
    let opened = app.open_code_symbol("mt069-code-save-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor after open_code_symbol, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "rich SaveManager idle before code-focused Save"
    );
    assert!(
        harness.state().last_editor_command().is_none(),
        "no code-host command before code-focused Save"
    );

    let fired = dispatch_palette_live(&mut harness, CMD_EDITOR_FILE_SAVE);
    assert!(fired, "code-focused Save dispatch reports an effect");
    harness.run_steps(2);

    assert_eq!(
        harness.state().last_editor_command(),
        Some(&handshake_native::code_editor::keymap::CodeEditorAction::Save),
        "MT-069: active code Save reaches the mounted code panel host-save channel"
    );
    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "MT-069: active code Save must NOT also dispatch the rich SaveManager"
    );
    assert!(
        !code_panel.buffer().to_string().is_empty(),
        "sanity: the mounted code panel remained live after Save dispatch"
    );
}

// ── AC-002 / PT-002: EDIT > Toggle Comment + Format Document reach the REAL editor transforms ───────────

#[test]
fn edit_toggle_comment_mutates_the_code_buffer() {
    use handshake_native::command_registry::{
        CMD_EDITOR_EDIT_FORMAT_DOCUMENT, CMD_EDITOR_EDIT_TOGGLE_COMMENT,
    };
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let opened = app.open_code_symbol("mt069-toggle-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor after open_code_symbol, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // The mounted code pane's seed buffer (CODE_EDITOR_SEED) is Rust ("rs"); its FIRST line is the
    // commented header `// Handshake native code editor (VS Code parity).`. The default caret is on line 0,
    // so a Toggle Comment acts on that line. Capture the live buffer before the toggle.
    let before = code_panel.buffer().to_string();
    let first_line_before = before.lines().next().unwrap_or_default().to_string();
    assert!(
        first_line_before.trim_start().starts_with("//"),
        "the seed's first line is a Rust line-comment: {first_line_before:?}"
    );

    // Dispatch EDIT > Toggle Comment through the SAME shell dispatcher a menu click reaches
    // (dispatch_palette_action -> dispatch_editor_command). This is NOT a bare repaint: the arm routes
    // `CodeEditorAction::ToggleComment` to the mounted panel's `dispatch_action`, which runs the MT-051
    // `line_ops::toggle_comment` transform on the caret's line.
    let fired = dispatch_palette_live(&mut harness, CMD_EDITOR_EDIT_TOGGLE_COMMENT);
    harness.run();
    assert!(
        fired,
        "Toggle Comment produced an observable effect (not a logged no-op)"
    );

    // AC-002: the buffer ACTUALLY changed — the menu Toggle Comment reached the real MT-051 transform, not
    // a fake-enabled no-op (RISK-003). Toggling the commented first line UNcomments it (the `// ` prefix is
    // stripped), so the buffer differs and the first line no longer leads with `//`.
    let after = code_panel.buffer().to_string();
    assert_ne!(
        after, before,
        "EDIT > Toggle Comment mutated the code buffer (MT-051 transform ran)"
    );
    let first_line_after = after.lines().next().unwrap_or_default().to_string();
    assert!(
        !first_line_after.trim_start().starts_with("//"),
        "Toggle Comment stripped the line-comment from the first line: {first_line_after:?}"
    );

    // Toggling again re-comments the line (VS Code all-or-nothing round-trip) — proves it is the real
    // reversible transform, not a one-way mutation.
    let fired2 = dispatch_palette_live(&mut harness, CMD_EDITOR_EDIT_TOGGLE_COMMENT);
    harness.run();
    assert!(fired2, "second Toggle Comment also fired");
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "a second Toggle Comment re-commented the line (real reversible MT-051 transform)"
    );

    // Format Document dispatches the REAL MT-050 format-request arm (arms textDocument/formatting; a no-op +
    // toast when no formatter is available — the honest MT-050 disabled path). It must dispatch without
    // panic and report an observable effect (the request was armed), never a fake-enabled bare repaint.
    let format_fired = dispatch_palette_live(&mut harness, CMD_EDITOR_EDIT_FORMAT_DOCUMENT);
    harness.run();
    assert!(
        format_fired,
        "Format Document dispatched the real MT-050 format request arm (not a no-op)"
    );
}

#[test]
fn edit_select_find_replace_dispatch_to_active_code_panel() {
    use handshake_native::command_registry::{
        CMD_EDITOR_EDIT_SELECT_ALL, CMD_EDITOR_FIND_FIND, CMD_EDITOR_FIND_REPLACE,
    };
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let opened = app.open_code_symbol("mt069-code-edit-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor after open_code_symbol, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    assert!(
        code_panel.selected_primary_text().is_none(),
        "precondition: the seed code buffer starts with a caret, not a selection"
    );
    let before = code_panel.buffer().to_string();
    let selected = dispatch_palette_live(&mut harness, CMD_EDITOR_EDIT_SELECT_ALL);
    harness.run();
    assert!(selected, "Select All reports a real code-panel dispatch");
    let (_, _, text) = code_panel
        .selected_primary_text()
        .expect("Select All selected the active code buffer");
    assert_eq!(
        text, before,
        "MT-069: EDIT > Select All selected the mounted code buffer, not only the generic bus"
    );

    let found = dispatch_palette_live(&mut harness, CMD_EDITOR_FIND_FIND);
    harness.run();
    assert!(found, "Find reports a real code-panel dispatch");
    assert!(
        code_panel.is_find_open(),
        "MT-069: EDIT > Find opened the mounted code editor find panel"
    );
    assert!(
        !code_panel.find_state().unwrap().show_replace,
        "Find opens the find panel without replace mode"
    );

    let replaced = dispatch_palette_live(&mut harness, CMD_EDITOR_FIND_REPLACE);
    harness.run();
    assert!(replaced, "Replace reports a real code-panel dispatch");
    assert!(
        code_panel.find_state().unwrap().show_replace,
        "MT-069: EDIT > Replace opens the mounted code editor find panel in replace mode"
    );
}

#[test]
fn edit_find_replace_dispatch_to_active_rich_panel() {
    use handshake_native::command_registry::{CMD_EDITOR_FIND_FIND, CMD_EDITOR_FIND_REPLACE};

    let (mut app, _rt) = editor_shell();
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_RICH_NOTE),
        "precondition: active pane is the mounted rich editor"
    );
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    let found = dispatch_palette_live(&mut harness, CMD_EDITOR_FIND_FIND);
    harness.run();
    assert!(found, "Find reports a real rich-panel dispatch");
    {
        let state = rich_state.lock().unwrap();
        let panel = state
            .find_replace
            .as_ref()
            .expect("Find opened the rich find panel");
        assert!(
            !panel.with_replace,
            "Find opens the rich panel without replace mode"
        );
    }

    let replaced = dispatch_palette_live(&mut harness, CMD_EDITOR_FIND_REPLACE);
    harness.run();
    assert!(replaced, "Replace reports a real rich-panel dispatch");
    {
        let state = rich_state.lock().unwrap();
        let panel = state
            .find_replace
            .as_ref()
            .expect("Replace keeps the rich find panel open");
        assert!(
            panel.with_replace,
            "Replace reveals the rich panel replace row"
        );
    }
}

// ── AC-002 / AC-004 / PT-003: EDIT > Undo routes through the MT-035 unified-undo scope (one stack) ──────

#[test]
fn edit_undo_routes_through_unified_undo_scope() {
    use handshake_native::code_editor::TextBuffer;

    let (app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // Seed a unified-undo entry on the SAME mounted code panel + focus the code pane, exactly like an edit
    // would, so the MT-035 unified scope holds one undoable action under the focused pane.
    let pane_id: PaneId = PaneId::from("pane-a");
    let before = code_panel.buffer().to_string();
    let after = format!("{before}\n// edited for MT-069 menu undo");
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
            "MT-069 menu undo edit",
        );
        assert_eq!(
            guard.local_undo_count(&pane_id),
            1,
            "one unified-undo entry seeded"
        );
        assert!(
            guard.undo_scope().can_undo_local(&pane_id),
            "can_undo_local true right after seed"
        );
        assert_eq!(
            guard.focus_owner(),
            Some(&pane_id),
            "focus owner is the code pane after seed"
        );
    }

    // The EDIT > Undo enable predicate is now true (can_undo). Open EDIT, confirm the leaf is ENABLED, and
    // click it by its UNIQUE author_id (labels are ambiguous — three "Undo" nodes exist in the live shell:
    // the menu leaf, a toolbar button, and a code-editor command button).
    harness.get_by_label("EDIT").click();
    harness.run_steps(2);
    let undo_node = live_author_nodes(&harness)
        .into_iter()
        .find(|(a, _, _)| a == "menu.edit.undo")
        .expect("Undo leaf present in open EDIT menu");
    assert!(
        !undo_node.2,
        "EDIT > Undo is ENABLED (the can_undo predicate read the seeded entry)"
    );
    click_author_id(&mut harness, "menu.edit.undo");
    harness.run_steps(2);

    // AC-004 / PT-003: the menu Undo popped the SAME MT-035 unified-undo scope entry the keyboard Undo
    // would — one shared stack. The entry is consumed and the panel reverted to `before`.
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert_eq!(
            guard.local_undo_count(&pane_id),
            0,
            "menu Undo popped the unified-undo entry (menu + keyboard share one MT-035 stack)"
        );
    }
    assert_eq!(
        code_panel.buffer().to_string(),
        before,
        "AC-004: menu Undo reverted the code pane via the MT-035 unified-undo scope"
    );
}

// ── AC-002 / PT-002: EDIT > Copy then Paste dispatch through the MT-031 shared clipboard ────────────────

#[test]
fn edit_copy_then_paste_dispatch_through_shared_clipboard() {
    use handshake_native::code_editor::cursor::Cursor;
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt031-copy-source");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor before Copy, got {opened:?}"
    );
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("hello from code\ntrailing");
    code_panel.set_cursors(vec![Cursor::selection(0, 5)]);
    let rich_state = app.mounted_rich_state();
    let before_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists before paste");
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // Click EDIT > Copy (by author_id): dispatches editor.edit.copy -> CMD_COPY on the bus, caching the
    // real mounted code selection into the MT-031 shared clipboard.
    harness.get_by_label("EDIT").click();
    harness.run();
    click_author_id(&mut harness, "menu.edit.copy");
    harness.run_steps(2);

    // AC-002: the MT-031 shared clipboard now holds the copied payload (the dispatch reached the real bus
    // clipboard handler — not inline editor logic in the menu).
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert_eq!(
            guard.clipboard_read_text().as_deref(),
            Some("hello"),
            "EDIT > Copy cached the real code selection into the MT-031 shared clipboard"
        );
    }

    assert!(
        dispatch_palette_live(&mut harness, CMD_VIEW_RICH_NOTE),
        "precondition: activate the mounted rich editor pane before Paste"
    );
    harness.run_steps(3);

    // With content on the clipboard, EDIT > Paste is now enabled and must mutate the focused rich buffer.
    harness.get_by_label("EDIT").click();
    harness.run();
    click_author_id(&mut harness, "menu.edit.paste");
    harness.run_steps(2);
    let after_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists after paste");
    assert_eq!(
        after_rich_text,
        format!("{before_rich_text}hello"),
        "EDIT > Paste inserted the shared clipboard payload into the focused rich editor buffer"
    );
}

#[test]
fn keyboard_ctrl_c_code_then_ctrl_v_rich_inserts_through_interaction_bus() {
    use handshake_native::code_editor::cursor::Cursor;
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt031-keyboard-copy-source");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor before Ctrl+C, got {opened:?}"
    );
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("hello from code\ntrailing");
    code_panel.set_cursors(vec![Cursor::selection(0, 5)]);
    let rich_state = app.mounted_rich_state();
    let before_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists before keyboard paste");
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    harness.event(egui::Event::Key {
        key: egui::Key::C,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    });
    harness.run_steps(3);
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert_eq!(
            guard.clipboard_read_text().as_deref(),
            Some("hello"),
            "Ctrl+C in the mounted code pane copied the real code selection into the MT-031 bus"
        );
    }

    assert!(
        dispatch_palette_live(&mut harness, CMD_VIEW_RICH_NOTE),
        "precondition: activate the mounted rich editor pane before Ctrl+V"
    );
    harness.run_steps(3);
    click_author_id(&mut harness, "rich-editor-surface");
    harness.run_steps(2);
    harness.event(egui::Event::Key {
        key: egui::Key::V,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    });
    harness.run_steps(3);

    let after_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists after keyboard paste");
    assert_eq!(
        after_rich_text,
        format!("{before_rich_text}hello"),
        "Ctrl+V in the mounted rich pane inserted the MT-031 bus clipboard into the rich buffer"
    );
}

#[test]
fn keyboard_ctrl_v_with_stale_code_selection_targets_focused_rich_pane() {
    use handshake_native::code_editor::cursor::Cursor;
    use handshake_native::interop::ClipboardPayload;
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt031-stale-selection-source");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor before stale-selection paste, got {opened:?}"
    );
    let code_panel = app.mounted_code_panel();
    let code_before = "alpha from code\ntrailing";
    code_panel.set_text(code_before);
    code_panel.set_cursors(vec![Cursor::selection(0, 5)]);
    let rich_state = app.mounted_rich_state();
    let before_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists before stale-selection paste");
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut guard = bus.lock().unwrap();
        guard.cache_clipboard(ClipboardPayload::PlainText("bus-payload".to_owned()));
    }

    click_author_id(&mut harness, "rich-editor-surface");
    harness.run_steps(2);
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut guard = bus.lock().unwrap();
        guard.set_focus_owner(PaneId::from("pane-b"));
    }
    harness.event(egui::Event::Key {
        key: egui::Key::V,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    });
    harness.run_steps(3);

    assert_eq!(
        code_panel.buffer().to_string(),
        code_before,
        "a stale code selection must not consume Ctrl+V or mutate the code buffer after rich focus"
    );
    let after_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists after stale-selection paste");
    assert_eq!(
        after_rich_text,
        format!("{before_rich_text}bus-payload"),
        "Ctrl+V targets the focused rich pane even when the code pane still has a selection"
    );
}

#[test]
fn palette_edit_copy_then_paste_inserts_into_focused_editor_buffer() {
    use handshake_native::code_editor::cursor::Cursor;
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt031-palette-copy-source");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor before palette Copy, got {opened:?}"
    );
    let code_panel = app.mounted_code_panel();
    code_panel.set_text("palette payload\ntrailing");
    code_panel.set_cursors(vec![Cursor::selection(0, 7)]);
    let rich_state = app.mounted_rich_state();
    let before_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists before palette paste");
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    harness.state_mut().open_command_palette();
    harness.run_steps(2);
    click_author_id(
        &mut harness,
        "command-palette.option.hs-editor-menu-edit-copy",
    );
    harness.run_steps(2);
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert_eq!(
            guard.clipboard_read_text().as_deref(),
            Some("palette"),
            "palette Copy row copied the mounted code selection into the MT-031 bus"
        );
    }

    click_author_id(&mut harness, "rich-editor-surface");
    harness.run_steps(2);
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut guard = bus.lock().unwrap();
        guard.set_focus_owner(PaneId::from("pane-b"));
    }
    harness.state_mut().open_command_palette();
    harness.run_steps(2);
    click_author_id(
        &mut harness,
        "command-palette.option.hs-editor-menu-edit-paste",
    );
    harness.run_steps(3);

    let after_rich_text = rich_state
        .lock()
        .unwrap()
        .block_plain_text(1)
        .expect("demo paragraph exists after palette paste");
    assert_eq!(
        after_rich_text,
        format!("{before_rich_text}palette"),
        "palette Paste row inserted the shared clipboard payload into the focused rich editor buffer"
    );
}

// ── AC-001 honesty: with NO editor pane, the editor menu items render DISABLED (not fake-enabled) ───────

#[test]
fn editor_menu_items_disabled_when_no_editor_pane() {
    let mut harness = shell_harness(plain_shell());
    harness.run();
    assert!(
        !harness.state().editor_available(),
        "no editor pane mounted in the plain shell"
    );

    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    let nodes = live_author_nodes(&harness);
    let save = nodes
        .iter()
        .find(|(a, _, _)| a == "menu.file.save")
        .expect("Save leaf present + addressable");
    assert!(
        save.2,
        "FILE > Save renders DISABLED when no editor pane is the target (honest, no fake-enable)"
    );
}

// ── AC-008 / PT-006: an editor pane present makes Save an ENABLED MenuItem with its WP-011 author_id ────

#[test]
fn file_save_is_enabled_accesskit_node_with_wp011_author_id() {
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt069-save-enabled-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor before opening FILE, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);
    assert!(
        harness.state().editor_available(),
        "an editor pane is the focusable target"
    );

    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    let nodes = live_author_nodes(&harness);
    // PT-006: the formerly-disabled `menu.file.save` is now an ENABLED node carrying its WP-011 author_id
    // (REUSED, not re-minted) with the MenuItem role.
    let save = nodes
        .iter()
        .find(|(a, _, _)| a == "menu.file.save")
        .unwrap_or_else(|| panic!("Save leaf missing from open FILE menu: {nodes:?}"));
    assert_eq!(save.1, "MenuItem", "Save leaf role is MenuItem");
    assert!(
        !save.2,
        "AC-008: Save is an ENABLED (pressable) AccessKit node when a code editor is active"
    );
}

#[test]
fn manual_documented_file_edit_go_rows_render_live_menu_items() {
    use handshake_native::manual_content_editors::agent_tool_rows;
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt069-manual-menu-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor after open_code_symbol, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    let manual_rows: std::collections::HashMap<&str, _> = agent_tool_rows()
        .into_iter()
        .map(|row| (row.author_id, row))
        .collect();
    for &author_id in handshake_native::top_menu_bar::EDITOR_MENU_LEAF_AUTHOR_IDS {
        let row = manual_rows
            .get(author_id)
            .unwrap_or_else(|| panic!("manual row missing for dynamic menu leaf {author_id}"));
        assert_eq!(
            row.mcp_tool, "click_widget",
            "manual row {author_id} must use the real click_widget tool"
        );
    }

    let mut rendered: std::collections::HashMap<String, (String, bool)> =
        std::collections::HashMap::new();
    for menu_label in ["FILE", "EDIT", "GO"] {
        harness.get_by_label(menu_label).click();
        harness.run();
        for (author_id, role, disabled) in live_author_nodes(&harness) {
            rendered.insert(author_id, (role, disabled));
        }
    }

    for &author_id in handshake_native::top_menu_bar::EDITOR_MENU_LEAF_AUTHOR_IDS {
        let (role, _disabled) = rendered.get(author_id).unwrap_or_else(|| {
            panic!("documented FILE/EDIT/GO menu leaf {author_id} did not render in live dropdowns")
        });
        assert_eq!(
            role, "MenuItem",
            "documented menu leaf {author_id} must be a live AccessKit MenuItem"
        );
    }
}

// ── MT-069 REMEDIATION: the GO-menu code-nav items are LIVE (registered + enabled) ──────────────────────

/// Pre-remediation these four items rendered DISABLED with a typed no-op (their owning commands were
/// unregistered — the old AC-003 honest-pending state). MT-069 REMEDIATION registered the code-nav
/// shell commands against the mounted panel, so with an editor pane mounted the GO items render
/// ENABLED, the pending set is EMPTY, and the dispatcher no longer treats them as pending.
#[test]
fn go_nav_items_render_enabled_and_registered() {
    use handshake_native::command_registry::{
        CMD_EDITOR_GO_TO_LINE, CMD_EDITOR_GO_TO_REFERENCES, CMD_EDITOR_GO_TO_SYMBOL,
    };
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt069-go-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor after open_code_symbol, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    harness.get_by_label("GO").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    // The four contract-named GO-nav ids are present, addressable by their stable command id, and
    // ENABLED (their owning code-nav shell commands are registered — MT-069 remediation).
    for go_id in [
        CMD_EDITOR_GO_TO_DEFINITION,
        CMD_EDITOR_GO_TO_REFERENCES,
        CMD_EDITOR_GO_TO_SYMBOL,
        CMD_EDITOR_GO_TO_LINE,
    ] {
        let found = nodes
            .iter()
            .find(|(a, _, _)| a == go_id)
            .unwrap_or_else(|| panic!("GO-nav item {go_id} missing from open GO menu: {nodes:?}"));
        assert!(
            !found.2,
            "MT-069: GO-nav item {go_id} renders ENABLED (owner registered against the mounted panel)"
        );
    }
    // No GO-nav id remains pending: the typed-pending set is empty and the dispatcher routes for real.
    assert!(
        EDITOR_GO_NAV_PENDING_IDS.is_empty(),
        "MT-069: the GO-nav pending set is EMPTY (all owners registered)"
    );
    assert!(
        !command_registry::is_go_nav_pending(CMD_EDITOR_GO_TO_DEFINITION),
        "MT-069: Go to Definition is no longer a pending no-op"
    );
}

// ── AC-005 / PT-004: the command-palette editor entries are now enabled + dispatch ─────────────────────

#[test]
fn palette_editor_entries_enabled_and_dispatch() {
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt069-palette-enabled-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted code editor before palette open, got {opened:?}"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(2);
    harness.state_mut().open_command_palette();
    harness.run_steps(2);

    // Filter to the editor Save palette row (an EditorMenu command). It is ENABLED now (the code editor is
    // the active target) and dispatchable — distinct from the rich-text "Bold" row which stays disabled.
    let nodes = live_author_nodes(&harness);
    let save_row = nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-file-save")
        .unwrap_or_else(|| panic!("editor Save palette row missing: {nodes:?}"));
    assert!(
        !save_row.2,
        "AC-005: the command-palette editor Save entry is ENABLED when the code editor is active"
    );
    // The Quick Switcher / quick-open editor entry is likewise present + enabled.
    let quick_open = nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-quick-open")
        .unwrap_or_else(|| panic!("Quick Open palette row missing: {nodes:?}"));
    assert!(
        !quick_open.2,
        "AC-005: the Quick Switcher palette entry is enabled"
    );
}

#[test]
fn palette_save_live_row_click_reaches_active_rich_save_manager() {
    let (mut app, _rt) = editor_shell();
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_RICH_NOTE),
        "precondition: activate the mounted rich editor pane"
    );
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(2);
    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "SaveManager idle before palette Save click"
    );

    harness.state_mut().open_command_palette();
    harness.run_steps(2);
    let nodes = live_author_nodes(&harness);
    let save_row = nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-file-save")
        .unwrap_or_else(|| panic!("palette Save row missing: {nodes:?}"));
    assert!(
        !save_row.2,
        "palette Save row is enabled for the active rich editor"
    );

    click_author_id(
        &mut harness,
        "command-palette.option.hs-editor-menu-file-save",
    );
    harness.run_steps(2);

    assert!(
        rich_state.lock().unwrap().save_is_in_flight(),
        "live command-palette Save row click reached the active rich SaveManager"
    );
}

#[test]
fn editor_menu_and_palette_rows_disable_when_mounted_editors_are_not_active() {
    let (mut app, _rt) = editor_shell();
    assert!(
        app.dispatch_palette_action_for_test(CMD_VIEW_GRAPH),
        "precondition: activate a non-editor Graph surface while the rich editor remains mounted"
    );
    let mut harness = shell_harness(app);
    harness.run_steps(3);
    assert!(
        harness.state().editor_available(),
        "editors are still mounted somewhere in the workspace"
    );

    harness.get_by_label("FILE").click();
    harness.run_steps(2);
    let file_nodes = live_author_nodes(&harness);
    for author_id in [
        "menu.file.save",
        "menu.file.save-as",
        "menu.file.export-html",
    ] {
        let node = file_nodes
            .iter()
            .find(|(a, _, _)| a == author_id)
            .unwrap_or_else(|| panic!("FILE leaf {author_id} missing: {file_nodes:?}"));
        assert!(
            node.2,
            "{author_id} is disabled when the active surface is non-editor"
        );
    }

    harness.get_by_label("EDIT").click();
    harness.run_steps(2);
    let edit_nodes = live_author_nodes(&harness);
    for author_id in ["menu.edit.find-replace", "menu.edit.select-all"] {
        let node = edit_nodes
            .iter()
            .find(|(a, _, _)| a == author_id)
            .unwrap_or_else(|| panic!("EDIT leaf {author_id} missing: {edit_nodes:?}"));
        assert!(
            node.2,
            "{author_id} is disabled when no editor target is active"
        );
    }

    harness.state_mut().open_command_palette();
    harness.run_steps(2);
    let palette_nodes = live_author_nodes(&harness);
    let save_row = palette_nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-file-save")
        .unwrap_or_else(|| panic!("palette Save row missing: {palette_nodes:?}"));
    assert!(
        save_row.2,
        "palette Save row is disabled when editors are mounted but not active"
    );
    let quick_open = palette_nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-quick-open")
        .unwrap_or_else(|| panic!("palette Quick Open row missing: {palette_nodes:?}"));
    assert!(
        !quick_open.2,
        "global Quick Open remains enabled without an active editor target"
    );
}

#[test]
fn palette_editor_entries_disabled_when_no_editor_pane() {
    let mut harness = shell_harness(plain_shell());
    harness.run();
    harness.state_mut().open_command_palette();
    harness.run_steps(2);

    let nodes = live_author_nodes(&harness);
    // With NO editor pane, the editor Save palette row renders DISABLED (the live predicate gates it) —
    // honest, no fake-enabled row.
    let save_row = nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-file-save")
        .unwrap_or_else(|| panic!("editor Save palette row missing: {nodes:?}"));
    assert!(
        save_row.2,
        "the editor Save palette entry is DISABLED when no editor pane is mounted (no fake-enable)"
    );
}

// ── AC-005 / PT-004: dispatching the Quick Switcher palette command opens the ONE WP-011 switcher ───────

#[test]
fn palette_quick_open_dispatch_opens_quick_switcher() {
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(2);
    assert!(
        !harness.state().quick_switcher_open(),
        "switcher closed initially"
    );

    // Dispatch the workbench quick-open command directly through the SAME shell dispatcher the palette Run
    // outcome calls (dispatch_palette_action -> dispatch_editor_command), proving the editor-menu palette
    // entry reaches a real handler (no fake command, no panic).
    let fired = dispatch_palette_live(&mut harness, CMD_WORKBENCH_QUICK_OPEN);
    harness.run();
    assert!(
        fired,
        "the quick-open editor command produced an observable effect"
    );
    assert!(
        harness.state().quick_switcher_open(),
        "AC-005: the Quick Switcher palette command opened the ONE WP-011 quick switcher"
    );
}

#[test]
fn palette_quick_open_live_row_click_opens_quick_switcher() {
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(2);
    harness.state_mut().open_command_palette();
    harness.run_steps(2);

    click_author_id(
        &mut harness,
        "command-palette.option.hs-editor-menu-quick-open",
    );
    harness.run_steps(2);

    assert!(
        harness.state().quick_switcher_open(),
        "live command-palette Quick Open row click opened the ONE WP-011 quick switcher"
    );
}

// ── AC-006 / PT-001: every formerly-disabled editor item dispatches WITHOUT panic on the live tree ──────

#[test]
fn every_editor_menu_command_dispatches_without_panic() {
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // Drive EVERY EditorMenu command id + every GO-nav pending id through the live shell dispatcher. None
    // may panic (AC-006 / MC-003): enabled commands produce an effect, pending GO-nav ids are a typed
    // logged no-op. This is the runtime half of the no-todo!()/unimplemented!()/panic!() proof.
    let menu_ids: Vec<&'static str> = command_registry::all_commands()
        .iter()
        .filter(|c| c.kind == CommandKind::EditorMenu)
        .map(|c| c.id)
        .collect();
    for id in menu_ids.iter().chain(EDITOR_GO_NAV_PENDING_IDS.iter()) {
        // Each dispatch returns a bool (effect or logged no-op) and must not panic.
        let _ = dispatch_palette_live(&mut harness, id);
        // Use step() (one frame), NOT run() (run-to-convergence): the Save dispatch arms a REAL MT-020
        // SaveManager save in flight, and the rich pane self-repaints while waiting for the (unreachable in
        // this headless test) backend result — run() would exceed max_steps on that legitimate loading
        // spinner. One frame is sufficient for the no-panic proof.
        harness.step();
    }
    // The shell is still alive + responsive after exercising every editor command path.
    assert!(
        harness.state().editor_available(),
        "shell intact after dispatching every editor command"
    );
}

// ── AC-003 / AC-006 / PT-005: static source scan — zero todo!()/unimplemented!()/panic!() on the wired
//    editor dispatch paths in top_menu_bar.rs + command_registry.rs + app.rs ────────────────────────────

#[test]
fn no_todo_unimplemented_or_panic_on_wired_handlers() {
    // The files MT-069 modifies (AC-007). Scan their source for the forbidden panic macros on the
    // wired editor dispatch paths. `panic_disabled`-style strings in comments/docs are excluded by checking
    // for the macro-invocation form (`todo!(` / `unimplemented!(` / `panic!(`).
    let files = [
        (
            "src/top_menu_bar.rs",
            include_str!("../src/top_menu_bar.rs"),
        ),
        (
            "src/command_registry.rs",
            include_str!("../src/command_registry.rs"),
        ),
        ("src/app.rs", include_str!("../src/app.rs")),
    ];
    for (name, src) in files {
        for (lineno, line) in src.lines().enumerate() {
            // Skip comment / doc lines — the forbidden macro NAMES legitimately appear in the AC-003
            // documentation explaining why they are NOT used. Only real CODE lines must be clean.
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") {
                continue;
            }
            for forbidden in ["todo!(", "unimplemented!(", "panic!("] {
                assert!(
                    !line.contains(forbidden),
                    "PT-005: '{forbidden}' must NOT appear on a code line in {name} (line {}): {line}",
                    lineno + 1
                );
            }
        }
    }
}

// ── AC-007: the menu/palette handlers route by command id ONLY (no inline editor logic) ────────────────

#[test]
fn menu_handlers_route_by_command_id_only() {
    // The top_menu_bar editor leaves emit `MenuBarAction::EditorCommand(<stable id>)` and the palette rows
    // carry stable command ids — both route to the shell's `dispatch_editor_command`. The menu source must
    // contain the `EditorCommand(` dispatch-by-id call form and must NOT call editor mutation functions
    // (e.g. `request_save`, `undo(`, `redo(`) directly inside the menu file (MC-001 / RISK-001).
    let menu_src = include_str!("../src/top_menu_bar.rs");
    assert!(
        menu_src.contains(
            "MenuBarAction::EditorCommand(crate::command_registry::CMD_EDITOR_FILE_SAVE)"
        ),
        "the FILE > Save leaf dispatches by command id (no inline editor logic)"
    );
    // No direct editor-mutation call sites in the menu file (it only routes by id).
    for forbidden in [
        "request_save_for_host(",
        ".undo(&",
        ".redo(&",
        "set_text(",
        "buffer_mut(",
    ] {
        assert!(
            !menu_src.contains(forbidden),
            "MC-001: the menu file must not contain inline editor logic ('{forbidden}')"
        );
    }
}

// ── AC-008 / HBR-VIS: AccessKit tree dump + screenshot of the wired menu to the EXTERNAL artifact root ──

#[test]
fn wired_menu_accesskit_dump_and_screenshot() {
    let _g = wgpu_guard();
    use handshake_native::quick_switcher::ShellNavigator;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_code_symbol("mt069-screenshot-active-code");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is code editor for enabled EDIT Copy proof, got {opened:?}"
    );
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1400.0, 900.0))
        .wgpu()
        .build_state(|ctx, app: &mut HandshakeApp| app.ui(ctx), app);
    harness.run_steps(3);

    // Open the EDIT menu so the wired editor leaves enter the live AccessKit tree, then dump the
    // editor-relevant nodes (AC-008): a tree snapshot proving the formerly-disabled items are now enabled
    // MenuItem nodes with their WP-011 author_ids.
    harness.get_by_label("EDIT").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    let edit_undo = nodes
        .iter()
        .find(|(a, _, _)| a == "menu.edit.undo")
        .unwrap_or_else(|| panic!("Undo leaf missing: {nodes:?}"));
    // With no seeded undo entry, Undo's predicate is false -> it is present but disabled (honest). Copy is
    // enabled because the active target is the code editor.
    assert_eq!(edit_undo.1, "MenuItem", "Undo leaf is a MenuItem node");
    let edit_copy = nodes
        .iter()
        .find(|(a, _, _)| a == "menu.edit.copy")
        .unwrap_or_else(|| panic!("Copy leaf missing: {nodes:?}"));
    assert!(
        !edit_copy.2,
        "EDIT > Copy is an enabled MenuItem node (code editor active)"
    );

    // wgpu screenshot of the wired EDIT menu -> the EXTERNAL artifact root ONLY. On a GPU host this saves a
    // PNG; absent an adapter, record an honest non-fatal note (the AccessKit proof above stands).
    match harness.render() {
        Ok(image) => {
            let (w, h) = (image.width(), image.height());
            assert!(w > 0 && h > 0, "rendered image is non-empty");
            let ext_dir = external_artifact_dir("wp-kernel-012-mt-069");
            let _ = std::fs::create_dir_all(&ext_dir);
            let png_path = ext_dir.join("MT-069-edit-menu-wired.png");
            let saved = image.save(&png_path).is_ok();
            let abs = std::fs::canonicalize(&png_path).unwrap_or(png_path.clone());
            println!(
                "MT-069 wired-menu screenshot: {w}x{h}, saved={saved} ({})",
                abs.display()
            );
            assert!(
                saved,
                "the wired-menu screenshot PNG saved to the external root"
            );
        }
        Err(e) => {
            println!(
                "BLOCKER(non-fatal): MT-069 wired-menu screenshot render unavailable (no wgpu adapter): \
                 {e}. The AccessKit enabled-node proof passed; the PNG is a GPU-host item."
            );
        }
    }
    assert_no_local_artifact_dir();
}

// ── Catalog sanity: the 22 menu command ids are catalog-enabled + the undo/redo ids are present ─────────

#[test]
fn editor_menu_catalog_has_the_contract_ids() {
    let menu: Vec<&str> = command_registry::all_commands()
        .iter()
        .filter(|c| c.kind == CommandKind::EditorMenu)
        .map(|c| c.id)
        .collect();
    for id in [
        CMD_EDITOR_FILE_SAVE,
        CMD_EDITOR_EDIT_UNDO,
        CMD_EDITOR_EDIT_REDO,
        command_registry::CMD_EDITOR_FOLD_AT_CURSOR,
        command_registry::CMD_EDITOR_UNFOLD_AT_CURSOR,
        command_registry::CMD_EDITOR_FOLD_ALL,
        command_registry::CMD_EDITOR_UNFOLD_ALL,
        CMD_WORKBENCH_QUICK_OPEN,
    ] {
        assert!(
            menu.contains(&id),
            "menu command id '{id}' present: {menu:?}"
        );
    }
    // MT-069 REMEDIATION: the original 22 plus the 9 GO code-navigation shell commands (definition /
    // references / workspace-symbol / line / next+prev diagnostic / back / forward / symbol-in-file),
    // plus the four code-folding commands rendered under EDIT.
    assert_eq!(menu.len(), 35, "exactly 35 EditorMenu commands wired");
}

// ── WP-KERNEL-012 MT-069 REMEDIATION: FILE > Export Document reaches the REAL export path ──────────────

/// An Export Document menu click yields REAL EXPORT BYTES through the MT-020 export path
/// (`export_document` -> the file-save sink -> `pending_file_save`), NOT a SaveManager save — the
/// pre-remediation build silently routed the four Export items to the plain SaveManager save (the
/// lying-enabled path). The kittest installs the synchronous `PathFileSaveSink` test sink (HBR-QUIET:
/// no OS dialog) and clicks the menu item through the SAME out-of-process AccessKit path a swarm agent
/// uses, then asserts the export bytes landed on disk and the MT-020 SaveManager state machine did NOT
/// move.
#[test]
fn file_export_click_yields_export_bytes_not_save_manager_state() {
    use handshake_native::quick_switcher::ShellNavigator;
    use handshake_native::rich_editor::save::conflict_ui::PathFileSaveSink;
    use std::collections::HashSet;

    let (mut app, _rt) = editor_shell();
    let opened = app.open_document("KRD-mt069-export-target");
    assert!(
        matches!(
            opened,
            handshake_native::quick_switcher::NavDispatchOutcome::Opened { .. }
        ),
        "precondition: active pane is the mounted rich editor after open_document, got {opened:?}"
    );
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let export_dir =
        external_artifact_dir("wp-kernel-012-mt-069").join(format!("menu-export-sink-{unique}"));
    assert!(
        !export_dir.exists(),
        "unique export directory must not exist before this test run: {}",
        export_dir.display()
    );
    app.set_export_save_sink_for_test(std::sync::Arc::new(PathFileSaveSink::new(&export_dir)));
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "SaveManager idle before the export click"
    );

    // Open FILE, click each Export Document format by its stable author_id (the swarm-agent click path).
    for (author_id, expected_ext) in [
        ("menu.file.export-html", "html"),
        ("menu.file.export-md", "md"),
        ("menu.file.export-txt", "txt"),
        ("menu.file.export-json", "json"),
    ] {
        let before: HashSet<std::path::PathBuf> = std::fs::read_dir(&export_dir)
            .map(|it| it.flatten().map(|entry| entry.path()).collect())
            .unwrap_or_default();
        harness.get_by_label("FILE").click();
        harness.run();
        click_author_id(&mut harness, author_id);
        harness.run_steps(2);
        let after: HashSet<std::path::PathBuf> = std::fs::read_dir(&export_dir)
            .map(|it| it.flatten().map(|entry| entry.path()).collect())
            .expect("export directory exists after click");
        let new_files: Vec<std::path::PathBuf> = after.difference(&before).cloned().collect();
        assert_eq!(
            new_files.len(),
            1,
            "{author_id} must write exactly one new file; before={before:?} after={after:?}"
        );
        let written = &new_files[0];
        assert_eq!(
            written.extension().and_then(|ext| ext.to_str()),
            Some(expected_ext),
            "{author_id} wrote the requested .{expected_ext} format"
        );
        let bytes = std::fs::read(written).expect("read export bytes");
        assert!(
            !bytes.is_empty(),
            "{author_id} wrote non-empty export bytes to {written:?}"
        );
        let text = String::from_utf8_lossy(&bytes);
        match expected_ext {
            "html" => assert!(
                text.contains("<!doctype html") || text.contains("<html"),
                "HTML export contains HTML markup: {text:?}"
            ),
            "json" => {
                let json: serde_json::Value =
                    serde_json::from_slice(&bytes).expect("JSON export parses");
                assert!(
                    json.get("content").is_some(),
                    "JSON export carries the ProseMirror content envelope: {json:?}"
                );
            }
            "md" | "txt" => assert!(
                !text.trim().is_empty(),
                ".{expected_ext} export contains document text"
            ),
            _ => unreachable!("covered expected_ext cases"),
        }
    }

    // The REAL export path ran: export bytes were written through the sink (export_document -> the
    // FileSaveSink -> pending_file_save, drained by the editor's own poll on the next frame).
    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(&export_dir)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    assert_eq!(
        entries.len(),
        4,
        "exactly one export per named FILE export item was written by the menu clicks: {entries:?}"
    );
    for entry in &entries {
        let bytes = std::fs::read(entry).expect("read export bytes");
        assert!(
            !bytes.is_empty(),
            "the export click produced REAL export bytes, got an empty file at {entry:?}"
        );
    }
    for expected_ext in ["html", "md", "txt", "json"] {
        assert!(
            entries.iter().any(|entry| {
                entry
                    .extension()
                    .map(|ext| ext == expected_ext)
                    .unwrap_or(false)
            }),
            "the named Export Document format .{expected_ext} was honored: {entries:?}"
        );
    }

    // NOT SaveManager state: the export click must NOT move the MT-020 SaveManager state machine (the
    // pre-remediation behavior this test locks out).
    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "MT-069: an Export click must NOT dispatch a SaveManager save (the lying-enabled path)"
    );

    assert_no_local_artifact_dir();
}

// ── WP-KERNEL-012 MT-009 REMEDIATION: the operator route renders a REAL diff ────────────────────────────

/// The palette `View: Diff / Merge` command (the operator route) CONSTRUCTS a real `DiffEditorPanel`
/// into the mounted `diff_slot` from the two REAL conflict buffers (local vs server) and the
/// Diff/Merge pane RENDERS it — pre-remediation, `diff_slot` was never populated, so the pane was a
/// permanent empty state. The conflict is installed on the mounted document's OWN SaveManager state
/// machine (the exact state a 409 save lands in), then the dispatch runs through the SAME
/// `dispatch_palette_action` arm a palette Run reaches.
#[test]
fn palette_diff_merge_route_renders_real_diff_from_save_conflict() {
    use handshake_native::command_registry::CMD_VIEW_DIFF_MERGE;
    use handshake_native::rich_editor::save::save_manager::{RichDocLoad, SaveState};

    let (app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // The mounted diff slot starts EMPTY (the pre-remediation permanent empty state).
    assert!(
        harness
            .state()
            .mounted_diff_slot()
            .lock()
            .unwrap()
            .is_none(),
        "diff_slot is empty before the operator route runs"
    );

    // Put the mounted document's SaveManager into a REAL save CONFLICT (the state a 409 lands in) —
    // the two content trees below are the two REAL buffers the shell diffs. The fresh mount carries no
    // SaveManager (it binds on document load), so the test installs the REAL production SaveManager
    // type first — the shell reads the SAME `save.state` machine either way.
    {
        let mut state = rich_state.lock().unwrap();
        if state.save.is_none() {
            state.save = Some(
                handshake_native::rich_editor::save::save_manager::SaveManager::new(
                    std::sync::Arc::new(handshake_native::backend_client::RichDocSaveBackend::new(
                        "http://127.0.0.1:1",
                    )),
                    None,
                    "KRD-conflict-mt009",
                    7,
                ),
            );
        }
        let save = state
            .save
            .as_mut()
            .expect("the mounted rich state carries the MT-020 SaveManager");
        save.state = SaveState::Conflict {
            server: Box::new(RichDocLoad {
                rich_document_id: "KRD-conflict-mt009".to_owned(),
                doc_version: 7,
                title: "conflicted-doc".to_owned(),
                content_json: Some(serde_json::json!({
                    "type": "doc",
                    "content": [{ "type": "paragraph", "content": [
                        { "type": "text", "text": "SERVER version of the paragraph" }
                    ]}]
                })),
                updated_at: None,
            }),
            local_content: serde_json::json!({
                "type": "doc",
                "content": [{ "type": "paragraph", "content": [
                    { "type": "text", "text": "LOCAL version of the paragraph" }
                ]}]
            }),
        };
    }

    // THE OPERATOR ROUTE: dispatch the palette `View: Diff / Merge` command through the same
    // production dispatcher a palette Run reaches.
    let fired = dispatch_palette_live(&mut harness, CMD_VIEW_DIFF_MERGE);
    assert!(
        fired,
        "palette View: Diff/Merge dispatched an observable effect"
    );
    harness.run_steps(3);

    // A REAL DiffEditorPanel was constructed into the mounted diff_slot from the conflict buffers…
    let diff_panel = harness.state().mounted_diff_slot().lock().unwrap().clone();
    assert!(
        diff_panel.is_some(),
        "MT-009: the operator route POPULATED diff_slot with a real DiffEditorPanel \
         (pre-remediation it was never populated)"
    );
    let diff_panel = diff_panel.unwrap();
    assert!(
        diff_panel
            .left_text()
            .contains("SERVER version of the paragraph"),
        "MT-009: conflict diff LEFT side is the server version, matching the conflict dialog"
    );
    assert!(
        diff_panel
            .right_text()
            .contains("LOCAL version of the paragraph"),
        "MT-009: conflict diff RIGHT side is the local/operator version, matching the conflict dialog"
    );

    // …and the Diff/Merge pane RENDERS it: the diff panel's stable author_id is live in the tree.
    let ids = live_author_nodes(&harness);
    assert!(
        ids.iter().any(|(a, _, _)| a == "diff_editor_panel"),
        "MT-009: a real diff RENDERS via the operator route (the 'diff_editor_panel' AccessKit node \
         is live); got {} author nodes",
        ids.len()
    );
}

#[test]
fn diff_merge_route_clears_stale_diff_when_no_current_conflict() {
    use handshake_native::command_registry::CMD_VIEW_DIFF_MERGE;
    use handshake_native::rich_editor::save::save_manager::{RichDocLoad, SaveState};

    let (app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    {
        let mut state = rich_state.lock().unwrap();
        state.save = Some(
            handshake_native::rich_editor::save::save_manager::SaveManager::new(
                std::sync::Arc::new(handshake_native::backend_client::RichDocSaveBackend::new(
                    "http://127.0.0.1:1",
                )),
                None,
                "KRD-stale-diff",
                7,
            ),
        );
        let save = state.save.as_mut().expect("SaveManager present");
        save.state = SaveState::Conflict {
            server: Box::new(RichDocLoad {
                rich_document_id: "KRD-stale-diff".to_owned(),
                doc_version: 7,
                title: "conflicted-doc".to_owned(),
                content_json: Some(serde_json::json!({
                    "type": "doc",
                    "content": [{ "type": "paragraph", "content": [
                        { "type": "text", "text": "SERVER stale conflict A" }
                    ]}]
                })),
                updated_at: None,
            }),
            local_content: serde_json::json!({
                "type": "doc",
                "content": [{ "type": "paragraph", "content": [
                    { "type": "text", "text": "LOCAL stale conflict A" }
                ]}]
            }),
        };
    }

    assert!(
        dispatch_palette_live(&mut harness, CMD_VIEW_DIFF_MERGE),
        "first diff route opens the conflict diff"
    );
    harness.run_steps(3);
    assert!(
        harness
            .state()
            .mounted_diff_slot()
            .lock()
            .unwrap()
            .is_some(),
        "first conflict populated the diff slot"
    );

    {
        let mut state = rich_state.lock().unwrap();
        state
            .save
            .as_mut()
            .expect("SaveManager still present")
            .state = SaveState::Idle;
    }

    assert!(
        dispatch_palette_live(&mut harness, CMD_VIEW_DIFF_MERGE),
        "no-conflict diff route still opens the honest empty Diff/Merge pane"
    );
    harness.run_steps(3);
    assert!(
        harness
            .state()
            .mounted_diff_slot()
            .lock()
            .unwrap()
            .is_none(),
        "no-conflict route clears the previous conflict diff instead of leaving stale content visible"
    );
    assert_live_author_id(
        &harness,
        "diff-merge-empty",
        "no-conflict View: Diff/Merge route",
    );
    let ids = live_author_ids(&harness);
    assert!(
        !ids.iter().any(|id| id == "diff_editor_panel"),
        "no-conflict route renders the empty state, not a stale diff panel"
    );
}

#[test]
fn conflict_diff_missing_server_content_renders_json_null() {
    use handshake_native::command_registry::CMD_VIEW_DIFF_MERGE;
    use handshake_native::rich_editor::save::save_manager::{RichDocLoad, SaveState};

    let (app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    {
        let mut state = rich_state.lock().unwrap();
        state.save = Some(
            handshake_native::rich_editor::save::save_manager::SaveManager::new(
                std::sync::Arc::new(handshake_native::backend_client::RichDocSaveBackend::new(
                    "http://127.0.0.1:1",
                )),
                None,
                "KRD-null-server",
                7,
            ),
        );
        let save = state.save.as_mut().expect("SaveManager present");
        save.state = SaveState::Conflict {
            server: Box::new(RichDocLoad {
                rich_document_id: "KRD-null-server".to_owned(),
                doc_version: 7,
                title: "server-null".to_owned(),
                content_json: None,
                updated_at: None,
            }),
            local_content: serde_json::json!({
                "type": "doc",
                "content": [{ "type": "paragraph", "content": [
                    { "type": "text", "text": "LOCAL content with null server" }
                ]}]
            }),
        };
    }

    assert!(
        dispatch_palette_live(&mut harness, CMD_VIEW_DIFF_MERGE),
        "diff route handles a server None content_json"
    );
    harness.run_steps(3);
    let diff_panel = harness
        .state()
        .mounted_diff_slot()
        .lock()
        .unwrap()
        .clone()
        .expect("diff panel installed");
    assert_eq!(
        diff_panel.left_text().trim(),
        "null",
        "server content_json=None renders as JSON null on the server side, not a blank invalid JSON buffer"
    );
    assert!(
        diff_panel
            .right_text()
            .contains("LOCAL content with null server"),
        "local side still carries the operator buffer"
    );
}

// ── WP-KERNEL-012 wave-6 (S6 item 1): VIEW > Open Editor Surfaces entries OPEN the mounted panes ─────────

/// Every "Open Editor Surfaces" VIEW-menu entry is present + addressable by its stable author_id (a swarm
/// agent / operator can SEE + click it out-of-process), each renders ENABLED (a real open, not a
/// dead/disabled/lying-enabled entry), and clicking a representative entry ("Open Knowledge Graph")
/// actually OPENS its pane: the module target pane's tab list gains the Graph View pane. Pre-wave-6 the
/// VIEW menu held ONLY theme/mode/drawer toggles — there was NO menu-bar path to open these surfaces.
#[test]
fn view_open_editor_surface_entries_open_real_panes() {
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // Open the VIEW menu so the Open Editor Surfaces leaves enter the live AccessKit tree.
    harness.get_by_label("VIEW").click();
    harness.run();
    let nodes = live_author_nodes(&harness);
    for author in [
        "menu.view.open-code-editor",
        "menu.view.open-rich-note",
        "menu.view.open-knowledge-graph",
        "menu.view.open-folders",
        "menu.view.open-tags",
        "menu.view.open-block-collections",
        "menu.view.open-canvas",
        "menu.view.open-loom-search",
        "menu.view.open-find-in-files",
        "menu.view.open-quick-switcher",
        "menu.view.open-daily-journal",
        "menu.view.open-diff-editor",
    ] {
        let n = nodes
            .iter()
            .find(|(a, _, _)| a == author)
            .unwrap_or_else(|| panic!("VIEW open-surface entry '{author}' missing: {nodes:?}"));
        assert_eq!(n.1, "MenuItem", "{author} is a MenuItem node");
        assert!(
            !n.2,
            "S6 item 1: {author} renders ENABLED (a real open, not a dead/disabled entry)"
        );
    }

    // Click "Open Knowledge Graph" through the out-of-process AccessKit path a swarm agent uses.
    click_author_id(&mut harness, "menu.view.open-knowledge-graph");
    harness.run_steps(2);

    // The pane OPENED: the module target pane's active tab list now contains the Graph View pane (opened
    // through the SAME `open_content_on_active_pane` primitive the palette `View: Graph` command uses).
    let graph_pane = PaneType::Placeholder("Graph View".to_owned());
    let target = harness
        .state()
        .active_pane()
        .cloned()
        .expect("an active pane exists after Open Knowledge Graph");
    let opened = harness
        .state()
        .tab_bar_states()
        .get(&target)
        .map(|bar| bar.tabs.iter().any(|t| t.pane_type == graph_pane))
        .unwrap_or(false);
    assert!(
        opened,
        "S6 item 1: Open Knowledge Graph OPENED the Graph View pane on the active work surface \
         (the active pane's tab list shows it) — not a no-op"
    );
}

/// The VIEW > Open Code Editor entry opens the CodeSymbol code-editor pane (the core native editor), a
/// second representative surface distinct from the placeholder-keyed Graph pane above (this one opens a
/// real `PaneType` variant).
#[test]
fn view_open_code_editor_opens_code_pane() {
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    harness.get_by_label("VIEW").click();
    harness.run();
    click_author_id(&mut harness, "menu.view.open-code-editor");
    harness.run_steps(2);

    let target = harness
        .state()
        .active_pane()
        .cloned()
        .expect("an active pane exists after Open Code Editor");
    let opened = harness
        .state()
        .tab_bar_states()
        .get(&target)
        .map(|bar| bar.tabs.iter().any(|t| t.pane_type == PaneType::CodeSymbol))
        .unwrap_or(false);
    assert!(
        opened,
        "S6 item 1: Open Code Editor opened the CodeSymbol code-editor pane on the active surface"
    );
}

#[test]
fn view_open_editor_surface_entries_render_each_real_surface() {
    let cases: Vec<(&str, Option<PaneType>, &str)> = vec![
        (
            "menu.view.open-code-editor",
            Some(PaneType::CodeSymbol),
            "code_editor_panel",
        ),
        (
            "menu.view.open-rich-note",
            Some(PaneType::LoomWikiPage),
            "rich-editor-root",
        ),
        (
            "menu.view.open-knowledge-graph",
            Some(PaneType::Placeholder(
                handshake_native::editor_pane_factories::GRAPH_VIEW_PANE_LABEL.to_owned(),
            )),
            "graph.zoom.in",
        ),
        (
            "menu.view.open-tags",
            Some(PaneType::Placeholder(
                handshake_native::editor_pane_factories::TAGS_PANE_LABEL.to_owned(),
            )),
            "tags.search",
        ),
        (
            "menu.view.open-block-collections",
            Some(PaneType::Placeholder(
                handshake_native::editor_pane_factories::BLOCK_COLLECTIONS_PANE_LABEL.to_owned(),
            )),
            "bcv.new-view",
        ),
        (
            "menu.view.open-canvas",
            Some(PaneType::AtelierEditor),
            "canvas.add-card",
        ),
        (
            "menu.view.open-loom-search",
            Some(PaneType::LoomSearchV2),
            "loom-search-v2.query",
        ),
        (
            "menu.view.open-find-in-files",
            Some(PaneType::FindInFiles),
            "find-in-files.query",
        ),
        (
            "menu.view.open-quick-switcher",
            None,
            "quick-switcher.dialog",
        ),
        (
            "menu.view.open-daily-journal",
            Some(PaneType::LoomDailyJournal),
            "daily-journal-panel",
        ),
        (
            "menu.view.open-diff-editor",
            Some(PaneType::Placeholder(
                handshake_native::editor_pane_factories::DIFF_MERGE_PANE_LABEL.to_owned(),
            )),
            "diff-merge-empty",
        ),
    ];

    for (menu_author_id, expected_pane, expected_surface_author_id) in cases {
        let (app, _rt) = editor_shell();
        let mut harness = shell_harness(app);
        harness.run_steps(3);

        harness.get_by_label("VIEW").click();
        harness.run();
        click_author_id(&mut harness, menu_author_id);
        harness.run_steps(4);

        let context = format!("VIEW open route {menu_author_id}");
        if let Some(pane_type) = expected_pane {
            assert_active_tab_contains(&harness, pane_type, &context);
        }
        assert_live_author_id(&harness, expected_surface_author_id, &context);
    }
}

#[test]
fn palette_view_open_commands_render_each_new_real_surface() {
    let cases: Vec<(&str, PaneType, &str)> = vec![
        (
            CMD_VIEW_CODE_EDITOR,
            PaneType::CodeSymbol,
            "code_editor_panel",
        ),
        (
            CMD_VIEW_RICH_NOTE,
            PaneType::LoomWikiPage,
            "rich-editor-root",
        ),
        (CMD_VIEW_CANVAS, PaneType::AtelierEditor, "canvas.add-card"),
        (
            CMD_VIEW_LOOM_SEARCH,
            PaneType::LoomSearchV2,
            "loom-search-v2.query",
        ),
        (
            CMD_VIEW_FIND_IN_FILES,
            PaneType::FindInFiles,
            "find-in-files.query",
        ),
    ];

    for (command_id, expected_pane, expected_surface_author_id) in cases {
        let (app, _rt) = editor_shell();
        let mut harness = shell_harness(app);
        harness.run_steps(3);

        assert!(
            dispatch_palette_live(&mut harness, command_id),
            "palette command {command_id} dispatches"
        );
        harness.run_steps(4);

        let context = format!("palette route {command_id}");
        assert_active_tab_contains(&harness, expected_pane, &context);
        assert_live_author_id(&harness, expected_surface_author_id, &context);
    }
}

#[test]
fn palette_view_open_rows_are_cataloged_live_and_clickable() {
    let cases: Vec<(&str, &str, Option<PaneType>, &str)> = vec![
        (
            CMD_VIEW_CODE_EDITOR,
            "command-palette.option.hs-view-palette-code-editor",
            Some(PaneType::CodeSymbol),
            "code_editor_panel",
        ),
        (
            CMD_VIEW_RICH_NOTE,
            "command-palette.option.hs-view-palette-rich-note",
            Some(PaneType::LoomWikiPage),
            "rich-editor-root",
        ),
        (
            CMD_VIEW_GRAPH,
            "command-palette.option.hs-view-palette-graph",
            Some(PaneType::Placeholder(
                handshake_native::editor_pane_factories::GRAPH_VIEW_PANE_LABEL.to_owned(),
            )),
            "graph.zoom.in",
        ),
        (
            CMD_VIEW_CANVAS,
            "command-palette.option.hs-view-palette-canvas",
            Some(PaneType::AtelierEditor),
            "canvas.add-card",
        ),
        (
            CMD_VIEW_LOOM_SEARCH,
            "command-palette.option.hs-view-palette-loom-search",
            Some(PaneType::LoomSearchV2),
            "loom-search-v2.query",
        ),
        (
            CMD_VIEW_FIND_IN_FILES,
            "command-palette.option.hs-view-palette-find-in-files",
            Some(PaneType::FindInFiles),
            "find-in-files.query",
        ),
        (
            CMD_WORKBENCH_QUICK_OPEN,
            "command-palette.option.hs-editor-menu-quick-open",
            None,
            "quick-switcher.dialog",
        ),
        (
            CMD_VIEW_JOURNAL,
            "command-palette.option.hs-view-palette-journal",
            Some(PaneType::LoomDailyJournal),
            "daily-journal-panel",
        ),
        (
            CMD_VIEW_DIFF_MERGE,
            "command-palette.option.hs-view-palette-diff-merge",
            Some(PaneType::Placeholder(
                handshake_native::editor_pane_factories::DIFF_MERGE_PANE_LABEL.to_owned(),
            )),
            "diff-merge-empty",
        ),
    ];

    let catalog = command_registry::all_commands();
    for (command_id, author_id, _, _) in &cases {
        let stable_id = author_id
            .strip_prefix("command-palette.option.")
            .expect("palette author_id uses row prefix");
        assert!(
            catalog
                .iter()
                .any(|cmd| cmd.id == *command_id && cmd.stable_id == stable_id),
            "palette catalog must include command {command_id} with stable_id {stable_id}"
        );
    }

    for (command_id, author_id, expected_pane, expected_surface_author_id) in cases {
        let (app, _rt) = editor_shell();
        let mut harness = shell_harness(app);
        harness.run_steps(3);
        harness.state_mut().open_command_palette();
        harness.run_steps(3);

        let row = live_author_nodes(&harness)
            .into_iter()
            .find(|(id, _, _)| id == author_id)
            .unwrap_or_else(|| panic!("live palette row {author_id} missing for {command_id}"));
        assert!(
            !row.2,
            "live palette row {author_id} for {command_id} is enabled"
        );

        click_author_id(&mut harness, author_id);
        harness.run_steps(4);

        let context = format!("clicked live palette row {author_id} for {command_id}");
        if let Some(expected_pane) = expected_pane {
            assert_active_tab_contains(&harness, expected_pane, &context);
        }
        assert_live_author_id(&harness, expected_surface_author_id, &context);
    }
}

// ── WP-KERNEL-012 wave-6 (S6 item 2 / MT-009 wire-up): the conflict dialog's Open-merge button opens a
//    REAL diff ─────────────────────────────────────────────────────────────────────────────────────────

/// The rich-editor save-conflict dialog's "Open merge" button (`conflict-open-merge`) is no longer a
/// no-op: clicking it raises a shell request that builds the REAL server-vs-local `DiffEditorPanel` into
/// the mounted `diff_slot` and opens the Diff/Merge pane. This drives the LIVE conflict WINDOW (rendered by
/// the mounted rich pane) and clicks the button through the same out-of-process AccessKit path a swarm
/// agent uses — NOT the palette route (that is proven separately above).
#[test]
fn conflict_dialog_open_merge_button_opens_real_diff() {
    use handshake_native::rich_editor::save::save_manager::{RichDocLoad, SaveState};

    let (app, _rt) = editor_shell();
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // The mounted diff slot starts EMPTY.
    assert!(
        harness
            .state()
            .mounted_diff_slot()
            .lock()
            .unwrap()
            .is_none(),
        "diff_slot is empty before the conflict-dialog Open-merge click"
    );

    // Put the mounted rich document's SaveManager into a REAL save CONFLICT (the state a 409 lands in) so
    // the mounted rich pane RENDERS the conflict window (with its 'conflict-open-merge' button).
    {
        let mut state = rich_state.lock().unwrap();
        if state.save.is_none() {
            state.save = Some(
                handshake_native::rich_editor::save::save_manager::SaveManager::new(
                    std::sync::Arc::new(handshake_native::backend_client::RichDocSaveBackend::new(
                        "http://127.0.0.1:1",
                    )),
                    None,
                    "KRD-conflict-s6",
                    7,
                ),
            );
        }
        let save = state.save.as_mut().expect("SaveManager present");
        save.state = SaveState::Conflict {
            server: Box::new(RichDocLoad {
                rich_document_id: "KRD-conflict-s6".to_owned(),
                doc_version: 7,
                title: "conflicted-doc".to_owned(),
                content_json: Some(serde_json::json!({
                    "type": "doc",
                    "content": [{ "type": "paragraph", "content": [
                        { "type": "text", "text": "SERVER version of the paragraph" }
                    ]}]
                })),
                updated_at: None,
            }),
            local_content: serde_json::json!({
                "type": "doc",
                "content": [{ "type": "paragraph", "content": [
                    { "type": "text", "text": "LOCAL version of the paragraph" }
                ]}]
            }),
        };
    }
    // Render so the conflict window mounts + its 'conflict-open-merge' button enters the live tree.
    harness.run_steps(2);
    let nodes = live_author_nodes(&harness);
    assert!(
        nodes.iter().any(|(a, _, _)| a == "conflict-open-merge"),
        "the conflict window's Open-merge button is live + addressable: {} author nodes",
        nodes.len()
    );

    // THE OPERATOR ACTION: click the conflict dialog's Open-merge button out-of-process. It raises the
    // shell request; `drive_editor_mounts` drains it next frame and builds the real diff.
    click_author_id(&mut harness, "conflict-open-merge");
    harness.run_steps(3);

    // A REAL DiffEditorPanel was constructed into the mounted diff_slot from the conflict buffers…
    assert!(
        harness
            .state()
            .mounted_diff_slot()
            .lock()
            .unwrap()
            .is_some(),
        "S6 item 2: the conflict-dialog Open-merge button POPULATED diff_slot with a real DiffEditorPanel \
         (pre-wave-6 the button was a no-op)"
    );
    // …and the Diff/Merge pane RENDERS it: the diff panel's stable author_id is live in the tree.
    let ids = live_author_nodes(&harness);
    assert!(
        ids.iter().any(|(a, _, _)| a == "diff_editor_panel"),
        "S6 item 2: a real diff RENDERS from the conflict-dialog Open-merge button (the 'diff_editor_panel' \
         AccessKit node is live); got {} author nodes",
        ids.len()
    );
}
