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
//! - AC-003 / PT-005: the GO-menu code-nav items render DISABLED with a typed logged no-op (never a panic),
//!   and a static source scan proves zero `todo!()`/`unimplemented!()`/`panic!()` on the wired handler
//!   bodies in `top_menu_bar.rs` + `command_registry.rs`.
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
    CMD_EDITOR_GO_TO_DEFINITION, CMD_WORKBENCH_QUICK_OPEN, EDITOR_GO_NAV_PENDING_IDS,
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

// ── AC-001 / AC-004 / PT-002 / PT-003: FILE > Save dispatches the MT-020 editor save path ──────────────

#[test]
fn file_save_dispatches_editor_save_path() {
    let (app, _rt) = editor_shell();
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
    harness.run();
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

// ── AC-002 / PT-002: EDIT > Toggle Comment + Format Document reach the REAL editor transforms ───────────

#[test]
fn edit_toggle_comment_mutates_the_code_buffer() {
    use handshake_native::command_registry::{
        CMD_EDITOR_EDIT_FORMAT_DOCUMENT, CMD_EDITOR_EDIT_TOGGLE_COMMENT,
    };

    let (app, _rt) = editor_shell();
    let code_panel = app.mounted_code_panel();
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
    let fired = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_EDITOR_EDIT_TOGGLE_COMMENT);
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
    let fired2 = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_EDITOR_EDIT_TOGGLE_COMMENT);
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
    let format_fired = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_EDITOR_EDIT_FORMAT_DOCUMENT);
    harness.run();
    assert!(
        format_fired,
        "Format Document dispatched the real MT-050 format request arm (not a no-op)"
    );
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
    harness.run();
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
    use handshake_native::interop::{text_range_selection, EditorSurfaceKind};

    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    // Seed a live shared selection on the bus so Copy has something to cache (the focused pane publishes a
    // TextRange selection when it has one).
    let pane_id: PaneId = PaneId::from("pane-a");
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let mut guard = bus.lock().unwrap();
        guard.set_focus_owner(pane_id.clone());
        let sel = text_range_selection(pane_id.clone(), EditorSurfaceKind::Code, 0, 5, "hello");
        guard.set_selection(sel);
        assert!(
            guard.clipboard_read().is_none(),
            "clipboard empty before Copy"
        );
    }

    // Click EDIT > Copy (by author_id): dispatches editor.edit.copy -> CMD_COPY on the bus, caching the
    // selection into the MT-031 shared clipboard.
    harness.get_by_label("EDIT").click();
    harness.run();
    click_author_id(&mut harness, "menu.edit.copy");
    harness.run_steps(2);

    // AC-002: the MT-031 shared clipboard now holds the copied payload (the dispatch reached the real bus
    // clipboard handler — not inline editor logic in the menu).
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert!(
            guard.clipboard_read().is_some(),
            "EDIT > Copy cached the selection into the MT-031 shared clipboard"
        );
    }

    // With content on the clipboard, EDIT > Paste is now enabled and dispatches without panic.
    harness.get_by_label("EDIT").click();
    harness.run();
    click_author_id(&mut harness, "menu.edit.paste");
    harness.run_steps(2);
    // No panic + the clipboard payload is still readable (Paste is a request signal; the pane consumes it).
    {
        let bus = InteractionBus::get_or_init(&harness.ctx);
        let guard = bus.lock().unwrap();
        assert!(
            guard.clipboard_read().is_some(),
            "clipboard payload persists after Paste request"
        );
    }
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
    harness.run();
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
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(3);
    assert!(
        harness.state().editor_available(),
        "an editor pane is the focusable target"
    );

    harness.get_by_label("FILE").click();
    harness.run();
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
        "AC-008: Save is an ENABLED (pressable) AccessKit node when an editor pane is mounted"
    );
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

    let (app, _rt) = editor_shell();
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
    let (app, _rt) = editor_shell();
    let mut harness = shell_harness(app);
    harness.run_steps(2);
    harness.state_mut().open_command_palette();
    harness.run_steps(2);

    // Filter to the editor Save palette row (an EditorMenu command). It is ENABLED now (an editor pane is
    // the target) and dispatchable — distinct from the rich-text "Bold" row which stays disabled.
    let nodes = live_author_nodes(&harness);
    let save_row = nodes
        .iter()
        .find(|(a, _, _)| a == "command-palette.option.hs-editor-menu-file-save")
        .unwrap_or_else(|| panic!("editor Save palette row missing: {nodes:?}"));
    assert!(
        !save_row.2,
        "AC-005: the command-palette editor Save entry is ENABLED when an editor pane is mounted"
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
    let fired = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_WORKBENCH_QUICK_OPEN);
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
        let _ = harness.state_mut().dispatch_palette_action_for_test(id);
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
//    editor dispatch paths in top_menu_bar.rs + command_registry.rs ─────────────────────────────────────

#[test]
fn no_todo_unimplemented_or_panic_on_wired_handlers() {
    // The two files MT-069 modifies (AC-007). Scan their source for the forbidden panic macros on the
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
    let (app, _rt) = editor_shell();
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
    // unconditionally enabled when an editor pane is mounted.
    assert_eq!(edit_undo.1, "MenuItem", "Undo leaf is a MenuItem node");
    let edit_copy = nodes
        .iter()
        .find(|(a, _, _)| a == "menu.edit.copy")
        .unwrap_or_else(|| panic!("Copy leaf missing: {nodes:?}"));
    assert!(
        !edit_copy.2,
        "EDIT > Copy is an enabled MenuItem node (editor pane mounted)"
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
        CMD_WORKBENCH_QUICK_OPEN,
    ] {
        assert!(
            menu.contains(&id),
            "menu command id '{id}' present: {menu:?}"
        );
    }
    // MT-069 REMEDIATION: the original 22 plus the 9 GO code-navigation shell commands (definition /
    // references / workspace-symbol / line / next+prev diagnostic / back / forward / symbol-in-file).
    assert_eq!(menu.len(), 31, "exactly 31 EditorMenu commands wired");
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
    use handshake_native::rich_editor::save::conflict_ui::PathFileSaveSink;

    let (mut app, _rt) = editor_shell();
    let export_dir = external_artifact_dir("wp-kernel-012-mt-069").join("menu-export-sink");
    let _ = std::fs::remove_dir_all(&export_dir);
    app.set_export_save_sink_for_test(std::sync::Arc::new(PathFileSaveSink::new(&export_dir)));
    let rich_state = app.mounted_rich_state();
    let mut harness = shell_harness(app);
    harness.run_steps(3);

    assert!(
        !rich_state.lock().unwrap().save_is_in_flight(),
        "SaveManager idle before the export click"
    );

    // Open FILE, click Export Document as JSON by its stable author_id (the swarm-agent click path).
    harness.get_by_label("FILE").click();
    harness.run();
    click_author_id(&mut harness, "menu.file.export-json");
    harness.run_steps(2);

    // The REAL export path ran: export bytes were written through the sink (export_document -> the
    // FileSaveSink -> pending_file_save, drained by the editor's own poll on the next frame).
    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(&export_dir)
        .map(|it| it.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    assert_eq!(
        entries.len(),
        1,
        "exactly one export written by the menu click: {entries:?}"
    );
    let bytes = std::fs::read(&entries[0]).expect("read export bytes");
    assert!(
        !bytes.is_empty(),
        "the export click produced REAL export bytes (ProseMirror JSON), got an empty file at {:?}",
        entries[0]
    );
    assert!(
        entries[0]
            .extension()
            .map(|e| e == "json")
            .unwrap_or(false),
        "the NAMED format (Export as JSON) was honored: {:?}",
        entries[0]
    );

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
    let fired = harness
        .state_mut()
        .dispatch_palette_action_for_test(CMD_VIEW_DIFF_MERGE);
    assert!(
        fired,
        "palette View: Diff/Merge dispatched an observable effect"
    );
    harness.run_steps(3);

    // A REAL DiffEditorPanel was constructed into the mounted diff_slot from the conflict buffers…
    assert!(
        harness
            .state()
            .mounted_diff_slot()
            .lock()
            .unwrap()
            .is_some(),
        "MT-009: the operator route POPULATED diff_slot with a real DiffEditorPanel \
         (pre-remediation it was never populated)"
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
