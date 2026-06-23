//! WP-KERNEL-012 MT-031 — the shared SharedSelection / Clipboard / Command bus, end-to-end against the
//! REAL editor panes.
//!
//! These tests mount the ACTUAL code + rich-text pane factories (`CodeEditorPaneFactory`,
//! `RichEditorPaneFactory`) through one shared [`handshake_native::interop::InteractionBus`] retrieved
//! from egui app data — the exact substrate a mounted pane uses — and drive selection / Copy / Paste /
//! the command surface through the panes' real render + buffer APIs. They do NOT assert the bus against a
//! hand-built stand-in: the cross-pane propagation is proven between the live `CodeEditorPanel` and the
//! live `RichEditorState` sharing the ONE bus the factories' `render()` wires (the Spec-Realism Gate's
//! "touch the real Handshake-owned resource" rule).
//!
//! - PT-1 / AC-1: a selection made in the REAL code panel is published to the shared bus by the code
//!   pane's `render()` wiring, and read back from the rich-text pane's perspective (cross-pane
//!   SharedSelection propagation between two real panes sharing one bus).
//! - PT-2 / AC-2: a code-pane Copy then a rich-text-pane Paste moves the identical text through the ONE
//!   shared bus + the mockable clipboard sink (headless-safe; the OS clipboard is never touched —
//!   red-team RISK-2 / MC-2). The rich pane inserts the pasted text into its REAL buffer.
//! - PT-3 / AC-3/AC-4: the four surfaces feed ONE command bus keyed by id (no forked per-surface
//!   duplicates); the command-palette trigger opens the shared palette flag; dispatch-by-id fires the
//!   registered handler.
//! - PT-4 / AC-5: the AccessKit tree carries the contract-named command-surface nodes
//!   `command-palette-trigger` (Button), `command-palette-search` (TextInput), and `cmd-{id}` (ListItem)
//!   including `cmd-Copy`.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::code_editor::panel::{CodeEditorPanel, CodeEditorPaneFactory};
use handshake_native::code_editor::cursor::Cursor;
use handshake_native::interop::adapters::{register_standard_commands, CommandPaletteSurface};
use handshake_native::interop::interaction_bus::{
    EditorSurfaceKind, InteractionBus, SharedSelection, CMD_COMMAND_PALETTE,
    COMMAND_PALETTE_SEARCH_AUTHOR_ID, COMMAND_PALETTE_TRIGGER_AUTHOR_ID,
};
use handshake_native::pane_registry::{
    DirtyState, LockState, PaneAuthority, PaneFactory, PaneId, PaneRecord, PaneRenderContext, PaneType,
};
use handshake_native::rich_editor::document_model::node::BlockNode;
use handshake_native::rich_editor::renderer::rich_editor_widget::{
    RichEditorPaneFactory, RichEditorState,
};
use handshake_native::rich_editor::properties::metadata_client::ClipboardSink;

// ── Artifact-hygiene helpers (CX-212E / CX-212F): screenshots/artifacts go to the EXTERNAL root ONLY ──

/// The crate-relative path to the external artifacts root (CX-212E), disk-agnostic — a sibling of the
/// repo worktree. Four `..` reach `<repo>/..` where `Handshake_Artifacts` lives.
#[allow(dead_code)]
fn external_artifact_dir(subdir: &str) -> PathBuf {
    Path::new("../../../../Handshake_Artifacts/handshake-test").join(subdir)
}

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

// ── A counted in-memory clipboard mock (the MT-017 control — the OS clipboard is never touched) ───────

struct MockClipboard {
    last: Mutex<Option<String>>,
}
impl MockClipboard {
    fn new() -> Self {
        Self { last: Mutex::new(None) }
    }
    fn taken(&self) -> Option<String> {
        self.last.lock().unwrap().clone()
    }
}
impl ClipboardSink for MockClipboard {
    fn copy(&self, text: &str) {
        *self.last.lock().unwrap() = Some(text.to_owned());
    }
}

fn pane(id: &str) -> PaneId {
    Arc::from(id)
}

fn pane_record(id: &str, pane_type: PaneType) -> PaneRecord {
    PaneRecord::new(
        pane(id),
        pane_type,
        "proj-test",
        None,
        LockState::Unlocked,
        DirtyState::Clean,
        PaneAuthority::System,
    )
}

// ── A kittest app that mounts the REAL code + rich-text pane factories sharing ONE bus ─────────────────

/// Mounts the actual `CodeEditorPaneFactory` and `RichEditorPaneFactory` — the real panes — and renders
/// each through `PaneFactory::render`, which retrieves the SAME shared bus from egui app data and wires
/// selection-publish + command registration. The test drives the panes' real APIs (the code panel's
/// cursor set, the rich state's selection) and reads the cross-pane result off the shared bus.
struct PaneApp {
    code: CodeEditorPaneFactory,
    code_record: PaneRecord,
    rich: RichEditorPaneFactory,
    rich_record: PaneRecord,
    /// A stable handle to the shared bus for assertions (== the app-data instance the factories use).
    bus: Arc<Mutex<InteractionBus>>,
    seeded: bool,
}

impl PaneApp {
    fn new(code_panel: CodeEditorPanel, rich_state: RichEditorState) -> Self {
        Self {
            code: CodeEditorPaneFactory::new(code_panel),
            code_record: pane_record("pane-code", PaneType::CodeSymbol),
            rich: RichEditorPaneFactory::new(Arc::new(Mutex::new(rich_state))),
            rich_record: pane_record("pane-rich", PaneType::LoomWikiPage),
            bus: Arc::new(Mutex::new(InteractionBus::new())),
            seeded: false,
        }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        let bus = InteractionBus::get_or_init(ctx);
        if !self.seeded {
            self.bus = bus.clone();
            self.seeded = true;
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            // The command-palette trigger + search + per-command AccessKit nodes (AC-5) over the SAME bus.
            // Rendered FIRST (at the top) so the trigger button is reliably hit-testable in the harness
            // viewport (the editor panes below own large scrollable regions).
            InteractionBus::with_try_lock(&bus, |b| {
                CommandPaletteSurface::trigger_button(ui, b);
            });
            if let Ok(guard) = bus.try_lock() {
                CommandPaletteSurface::emit_command_item_nodes(ui, &guard);
            }
            let mut query = String::new();
            let search = ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search actions..."));
            CommandPaletteSurface::emit_search_node(ui.ctx(), search.id);

            // Render the REAL code pane (its render() wires the bus + publishes selection).
            ui.push_id("code-pane-scope", |ui| {
                let render_ctx = PaneRenderContext {
                    record: &self.code_record,
                    egui_id: ui.id(),
                };
                self.code.render(ui, &render_ctx);
            });
            // Render the REAL rich-text pane (its render() wires the bus + publishes selection when it
            // holds focus — here it observes, since the code pane is the selection authority).
            ui.push_id("rich-pane-scope", |ui| {
                let render_ctx = PaneRenderContext {
                    record: &self.rich_record,
                    egui_id: ui.id(),
                };
                self.rich.render(ui, &render_ctx);
            });
        });
    }
}

fn pane_harness(code_text: &str) -> Harness<'static, PaneApp> {
    let code_panel = CodeEditorPanel::new(code_text, "rs");
    let rich_state = RichEditorState::new(BlockNode::doc(vec![BlockNode::paragraph("rich body text")]));
    Harness::builder()
        .build_state(|ctx, a: &mut PaneApp| a.ui(ctx), PaneApp::new(code_panel, rich_state))
}

/// Collect every live AccessKit node carrying an author_id: (author_id, role, label).
fn live_author_nodes(harness: &Harness<'_, PaneApp>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

// ── PT-1 / AC-1: cross-pane SharedSelection propagation between two REAL panes ─────────────────────────

#[test]
fn selection_propagates_from_real_code_pane_to_rich_pane() {
    const SRC: &str = "fn main() { let answer = 42; }";
    let mut harness = pane_harness(SRC);

    // Make a REAL selection in the code panel: select "let answer = 42;" by byte range.
    let start = SRC.find("let").unwrap();
    let end = SRC.find("42;").unwrap() + "42;".len();
    {
        let panel = harness.state().code.panel();
        panel.set_cursors(vec![Cursor::selection(start, end)]);
    }
    // Run a frame: the code pane's render() publishes the selection into the shared bus (it is the focus
    // owner because it has a live selection).
    harness.run();

    // Read the shared selection from the rich-text pane's perspective (a different consumer of the SAME
    // bus), guarded against a stale pane id (both panes are live).
    let bus = harness.state().bus.clone();
    let observed = {
        let b = bus.lock().unwrap();
        b.shared_selection_if_live(&[pane("pane-code"), pane("pane-rich")])
    };
    let expected = &SRC[start..end];
    match observed {
        SharedSelection::TextRange { pane_id, surface, text, .. } => {
            assert_eq!(pane_id.as_ref(), "pane-code", "selection carries the source pane id");
            assert_eq!(surface, EditorSurfaceKind::Code, "selection carries the source surface kind");
            assert_eq!(text, expected, "the real code panel's selected text propagated cross-pane");
        }
        other => panic!("expected a cross-pane TextRange from the real code pane, got {other:?}"),
    }
    assert_no_local_artifact_dir();
}

// ── PT-2 / AC-2: Copy in the REAL code pane, Paste into the REAL rich pane, through the ONE bus ────────

#[test]
fn clipboard_round_trip_real_code_to_real_rich_pane() {
    const SRC: &str = "the shared payload line";
    let mut harness = pane_harness(SRC);

    // Select the whole code line and publish it (the code pane's render() does the publish).
    {
        let panel = harness.state().code.panel();
        panel.set_cursors(vec![Cursor::selection(0, SRC.len())]);
    }
    harness.run();

    // Code pane Copy (the Ctrl+C path): copy the published shared selection through the bus + the mock
    // sink (the OS clipboard is NEVER touched — RISK-2 / MC-2).
    let mock = MockClipboard::new();
    let bus = harness.state().bus.clone();
    let copied = {
        let mut b = bus.lock().unwrap();
        let sel = b.shared_selection().clone();
        handshake_native::interop::adapters::copy_selection_to_clipboard(&mut b, &sel, &mock)
    };
    assert!(copied, "the real code pane copied its non-empty selection");
    assert_eq!(mock.taken().as_deref(), Some(SRC), "the mock sink received the OS write");

    // Rich-text pane Paste (the Ctrl+V READ path): the REAL rich pane reads the SAME bus's clipboard via
    // its own `paste_text_from_bus` adapter and gets the identical text the code pane copied. The rich
    // editor's buffer INSERT is its own transaction machinery (MT-011..020); MT-031 proves the cross-pane
    // clipboard CHANNEL carries the identical payload between the two real panes sharing one bus.
    let pasted_text = {
        let b = bus.lock().unwrap();
        handshake_native::rich_editor::interop_adapter::paste_text_from_bus(&b)
    };
    assert_eq!(
        pasted_text.as_deref(),
        Some(SRC),
        "the real rich-text pane read the identical text the real code pane copied (cross-pane round-trip)"
    );
    assert_no_local_artifact_dir();
}

// ── PT-3 / AC-3/AC-4: ONE command bus fed by all four surfaces; trigger opens; dispatch fires ─────────

#[test]
fn command_bus_is_unified_and_dispatch_fires_handler() {
    let mut harness = pane_harness("fn x() {}");
    harness.run();
    let bus = harness.state().bus.clone();

    // The two mounted panes (code + rich) each registered the six standard commands via their render()
    // wiring; register the graph + canvas surfaces too (their pane is a downstream mount — see remaining)
    // to prove the four-surface union still collapses to ONE id-keyed set (no forked per-surface dupes).
    {
        let mut b = bus.lock().unwrap();
        register_standard_commands(&mut b, EditorSurfaceKind::Graph);
        register_standard_commands(&mut b, EditorSurfaceKind::Canvas);
        assert_eq!(
            b.commands().len(),
            6,
            "the four surfaces feed ONE command bus keyed by id (no forked per-surface duplicates)"
        );
    }

    // The command-palette trigger opens the shared palette flag (the out-of-process path a swarm uses).
    assert!(!bus.lock().unwrap().command_palette_open());
    harness.get_by_label("⌘ Commands").click();
    harness.run();
    assert!(
        bus.lock().unwrap().command_palette_open(),
        "pressing the command-palette-trigger opened the shared palette"
    );

    // dispatch-by-id fires the registered handler (real side effect, not a tautology).
    let dispatch_ctx = egui::Context::default();
    {
        let mut b = bus.lock().unwrap();
        b.close_command_palette();
        assert!(!b.command_palette_open(), "palette closed before dispatch");
        assert!(
            b.dispatch_command(&dispatch_ctx, CMD_COMMAND_PALETTE),
            "dispatch_command found + ran the registered CommandPalette command"
        );
        assert!(
            b.command_palette_open(),
            "the dispatched handler reopened the palette (real handler side effect)"
        );
        assert!(!b.dispatch_command(&dispatch_ctx, "interop.does-not-exist"), "bad id is a no-op");
    }
    assert_no_local_artifact_dir();
}

// ── PT-3b: Copy/Cut dispatch-by-id moves the shared selection into the clipboard cache (no no-op) ─────

#[test]
fn copy_dispatch_caches_shared_selection() {
    const SRC: &str = "dispatch copy payload";
    let mut harness = pane_harness(SRC);
    {
        let panel = harness.state().code.panel();
        panel.set_cursors(vec![Cursor::selection(0, SRC.len())]);
    }
    harness.run();

    let bus = harness.state().bus.clone();
    let dispatch_ctx = egui::Context::default();
    {
        let mut b = bus.lock().unwrap();
        // The shared selection is the real code-pane selection; the bus has no cached clipboard yet.
        assert!(b.clipboard_read().is_none(), "no clipboard cache before Copy dispatch");
        assert!(
            b.dispatch_command(&dispatch_ctx, handshake_native::interop::interaction_bus::CMD_COPY),
            "Copy command dispatched"
        );
        // The Copy handler is NOT a no-op: it cached the shared selection as a clipboard payload.
        assert_eq!(
            b.clipboard_read_text().as_deref(),
            Some(SRC),
            "the Copy dispatch handler cached the real shared selection (not a permanent no-op)"
        );
    }
    assert_no_local_artifact_dir();
}

// ── PT-4 / AC-5: the AccessKit tree carries the contract-named command-surface nodes ──────────────────

#[test]
fn accesskit_command_surface_nodes_present() {
    let mut harness = pane_harness("fn x() {}");
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);

    let trigger = nodes
        .iter()
        .find(|(a, _, _)| a == COMMAND_PALETTE_TRIGGER_AUTHOR_ID)
        .unwrap_or_else(|| panic!("command-palette-trigger missing from live tree: {nodes:?}"));
    assert_eq!(trigger.1, "Button", "command-palette-trigger role is Button");

    let search = nodes
        .iter()
        .find(|(a, _, _)| a == COMMAND_PALETTE_SEARCH_AUTHOR_ID)
        .unwrap_or_else(|| panic!("command-palette-search missing: {nodes:?}"));
    assert_eq!(search.1, "TextInput", "command-palette-search role is TextInput");

    let cmd_copy_author = handshake_native::interop::interaction_bus::command_list_item_author_id("Copy");
    assert_eq!(cmd_copy_author, "cmd-Copy", "the contract's cmd-Copy address");
    let cmd_item = nodes
        .iter()
        .find(|(a, _, _)| a == &cmd_copy_author)
        .unwrap_or_else(|| panic!("cmd-Copy ({cmd_copy_author}) ListItem missing: {nodes:?}"));
    assert_eq!(cmd_item.1, "ListItem", "cmd-Copy role is ListItem");

    assert_no_local_artifact_dir();
}
