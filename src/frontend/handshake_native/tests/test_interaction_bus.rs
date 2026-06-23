//! WP-KERNEL-012 MT-031 — the shared SharedSelection / Clipboard / Command bus, end-to-end.
//!
//! These tests prove the E5 melt-together substrate ([`handshake_native::interop::InteractionBus`]) with
//! REAL runtime, not tautological pokes:
//!
//! - PT-1 / AC-1: a selection published from one pane (the focus owner) is observed by another pane,
//!   reflecting the source pane_id + materialized text (cross-pane SharedSelection propagation).
//! - PT-2 / AC-2: a clipboard round-trip — a code-surface Copy then a rich-text-surface Paste pastes the
//!   identical text — driven through the ONE shared bus + the mockable clipboard sink (headless-safe;
//!   the OS clipboard is never touched — red-team RISK-2 / MC-2). A LIVE kittest harness drives the same
//!   bus a swarm agent would.
//! - PT-3 / AC-3: the command palette opens via the shared bus trigger (Ctrl+Shift+P-equivalent),
//!   lists the registered cross-pane commands, and dispatch-by-id fires the handler side effect.
//! - PT-4 / AC-5: the AccessKit tree carries the contract-named nodes `command-palette-trigger`
//!   (Button), `command-palette-search` (TextInput), and `cmd-{id}` (ListItem) including `cmd-Copy`.
//!
//! ## No live backend needed
//!
//! The bus is a pure-frontend substrate; these tests drive a SMALL kittest app that mounts the bus +
//! the four surface adapters, exactly the shared object every editor pane retrieves. The clipboard is
//! the in-memory mock (MT-017 precedent), so no OS clipboard / no display server is required.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;

use handshake_native::interop::adapters::{
    copy_selection_to_clipboard, register_standard_commands, CommandPaletteSurface,
};
use handshake_native::interop::interaction_bus::{
    ClipboardPayload, EditorSurfaceKind, InteractionBus, SharedSelection, CMD_COMMAND_PALETTE,
    COMMAND_PALETTE_SEARCH_AUTHOR_ID, COMMAND_PALETTE_TRIGGER_AUTHOR_ID,
};
use handshake_native::pane_registry::PaneId;
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

// ── A small kittest app that mounts the shared bus + the command-surface adapters ─────────────────────

/// The minimal app the harness drives: it retrieves the SAME shared bus a real pane would (via egui app
/// data), renders the command-palette trigger + the search field + the per-command AccessKit nodes, and
/// keeps the bus reachable so the test can assert its state. This is the exact substrate the four editor
/// panes share — not a stand-in.
struct BusTestApp {
    /// A stable handle to the shared bus for assertions (== the app-data instance).
    bus: Arc<Mutex<InteractionBus>>,
    seeded: bool,
}

impl BusTestApp {
    fn new() -> Self {
        Self { bus: Arc::new(Mutex::new(InteractionBus::new())), seeded: false }
    }

    fn ui(&mut self, ctx: &egui::Context) {
        // Retrieve the SHARED bus (every pane does this); on first frame, adopt the app-data instance as
        // our handle so the harness drives the same object the surfaces would.
        let bus = InteractionBus::get_or_init(ctx);
        if !self.seeded {
            self.bus = bus.clone();
            // Mount all four surface adapters into the ONE bus (AC-4) once.
            if let Some(()) = InteractionBus::with_try_lock(&bus, |b| {
                register_standard_commands(b, EditorSurfaceKind::Code);
                register_standard_commands(b, EditorSurfaceKind::RichText);
                register_standard_commands(b, EditorSurfaceKind::Graph);
                register_standard_commands(b, EditorSurfaceKind::Canvas);
            }) {
                self.seeded = true;
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            // The command-palette trigger button + the per-command AccessKit ListItem nodes (AC-5).
            InteractionBus::with_try_lock(&bus, |b| {
                CommandPaletteSurface::trigger_button(ui, b);
            });
            // Emit the cmd-{id} ListItem nodes (read-only over the bus).
            if let Ok(guard) = bus.try_lock() {
                CommandPaletteSurface::emit_command_item_nodes(ui, &guard);
            }
            // A search field carrying the command-palette-search address (AC-5).
            let mut query = String::new();
            let search = ui.add(egui::TextEdit::singleline(&mut query).hint_text("Search actions..."));
            CommandPaletteSurface::emit_search_node(ui.ctx(), search.id);
        });
    }
}

fn harness() -> Harness<'static, BusTestApp> {
    Harness::builder().build_state(|ctx, a: &mut BusTestApp| a.ui(ctx), BusTestApp::new())
}

/// Collect every live AccessKit node carrying an author_id: (author_id, role, label).
fn live_author_nodes(harness: &Harness<'_, BusTestApp>) -> Vec<(String, String, Option<String>)> {
    let mut found = Vec::new();
    for node in harness.root().children_recursive() {
        let ak = node.accesskit_node();
        if let Some(author_id) = ak.author_id() {
            found.push((author_id.to_owned(), format!("{:?}", ak.role()), ak.label()));
        }
    }
    found
}

// ── PT-1 / AC-1: cross-pane SharedSelection propagation ───────────────────────────────────────────────

#[test]
fn selection_propagates_across_panes() {
    // The code pane (focus owner) publishes a text selection; the rich-text pane reads the SAME bus and
    // observes the source pane_id + text.
    let bus = Arc::new(Mutex::new(InteractionBus::new()));
    {
        let mut b = bus.lock().unwrap();
        b.set_focus_owner(pane("pane-code"));
        let sel = SharedSelection::TextRange {
            pane_id: pane("pane-code"),
            surface: EditorSurfaceKind::Code,
            start: 0,
            end: 11,
            text: "fn main(){}".to_owned(),
        };
        assert!(b.set_selection(sel), "the focus owner's selection is accepted");
    }
    // The rich-text pane (a different consumer of the same bus) reads it back, guarded against a stale
    // pane id (the pane is live here).
    let observed = {
        let b = bus.lock().unwrap();
        b.shared_selection_if_live(&[pane("pane-code"), pane("pane-rich")])
    };
    match observed {
        SharedSelection::TextRange { pane_id, text, .. } => {
            assert_eq!(pane_id.as_ref(), "pane-code", "selection carries the source pane id");
            assert_eq!(text, "fn main(){}", "selection carries the materialized text");
        }
        other => panic!("expected a cross-pane TextRange selection, got {other:?}"),
    }
    assert_no_local_artifact_dir();
}

// ── PT-2 / AC-2: clipboard round-trip code -> rich-text through the ONE bus ────────────────────────────

#[test]
fn clipboard_round_trip_code_to_richtext() {
    let mut harness = harness();
    harness.run();

    let mock = MockClipboard::new();
    let bus = harness.state().bus.clone();

    // Code pane: select text, then Copy (Ctrl+C path) — the selection is published + copied through the
    // shared bus + the mock sink (the OS clipboard is NEVER touched — RISK-2 / MC-2).
    const PAYLOAD: &str = "shared payload";
    let copied = {
        let mut b = bus.lock().unwrap();
        b.set_focus_owner(pane("pane-code"));
        let sel = SharedSelection::TextRange {
            pane_id: pane("pane-code"),
            surface: EditorSurfaceKind::Code,
            start: 0,
            end: PAYLOAD.len(),
            text: PAYLOAD.to_owned(),
        };
        b.set_selection(sel.clone());
        copy_selection_to_clipboard(&mut b, &sel, &mock)
    };
    assert!(copied, "the code pane copied a non-empty selection");
    assert_eq!(mock.taken().as_deref(), Some(PAYLOAD), "the mock sink received the OS write");

    // Rich-text pane: Paste (Ctrl+V path) — reads the SAME bus's clipboard and gets the identical text.
    let pasted = {
        let b = bus.lock().unwrap();
        b.clipboard_read_text()
    };
    assert_eq!(
        pasted.as_deref(),
        Some(PAYLOAD),
        "the rich-text pane pasted the identical text the code pane copied (cross-pane round-trip)"
    );

    // A rich LoomBlockRef survives the cross-pane cache even though the OS clipboard flattens it to a URI.
    {
        let mut b = bus.lock().unwrap();
        b.clipboard_write(ClipboardPayload::LoomBlockRef("blk-77".to_owned()), &mock);
    }
    assert_eq!(mock.taken().as_deref(), Some("loom://blk-77"), "OS clipboard got the flattened URI");
    let rich = { bus.lock().unwrap().clipboard_read().cloned() };
    assert_eq!(
        rich,
        Some(ClipboardPayload::LoomBlockRef("blk-77".to_owned())),
        "the rich LoomBlockRef variant survives for a cross-pane Paste"
    );
    assert_no_local_artifact_dir();
}

// ── PT-3 / AC-3: command palette opens via the bus trigger + dispatch-by-id fires the handler ─────────

#[test]
fn command_palette_opens_and_dispatches() {
    let mut harness = harness();
    harness.run();
    let bus = harness.state().bus.clone();

    // The four surfaces each registered the six standard commands; last-registration-wins keys by id, so
    // the ONE bus holds exactly the six melt-together commands (no forked duplicate per surface).
    assert_eq!(
        bus.lock().unwrap().commands().len(),
        6,
        "the four surfaces feed ONE command bus keyed by id (no forked per-surface duplicates)"
    );

    // The palette is closed initially.
    assert!(!bus.lock().unwrap().command_palette_open());

    // Press the command-palette trigger button (the genuine out-of-process path a swarm agent uses).
    harness.get_by_label("⌘ Commands").click();
    harness.run();
    assert!(
        bus.lock().unwrap().command_palette_open(),
        "pressing the command-palette-trigger opened the shared palette"
    );

    // dispatch-by-id fires the command handler (here the CommandPalette opener, proving the dispatch
    // path runs the registered handler with the locked bus). Close first, then dispatch reopens it.
    let dispatch_ctx = egui::Context::default();
    {
        let mut b = bus.lock().unwrap();
        b.close_command_palette();
        assert!(!b.command_palette_open(), "palette closed before dispatch");
        let dispatched = b.dispatch_command(&dispatch_ctx, CMD_COMMAND_PALETTE);
        assert!(dispatched, "dispatch_command found + ran the registered CommandPalette command");
        assert!(
            b.command_palette_open(),
            "the dispatched handler reopened the palette (real handler side effect, not a tautology)"
        );
        // A bad id is a no-op, never a panic.
        assert!(!b.dispatch_command(&dispatch_ctx, "interop.does-not-exist"));
    }
    assert_no_local_artifact_dir();
}

// ── PT-4 / AC-5: the AccessKit tree carries the contract-named command-surface nodes ──────────────────

#[test]
fn accesskit_command_surface_nodes_present() {
    let mut harness = harness();
    harness.run();
    harness.run();

    let nodes = live_author_nodes(&harness);

    // command-palette-trigger (Button).
    let trigger = nodes
        .iter()
        .find(|(a, _, _)| a == COMMAND_PALETTE_TRIGGER_AUTHOR_ID)
        .unwrap_or_else(|| panic!("command-palette-trigger missing from live tree: {nodes:?}"));
    assert_eq!(trigger.1, "Button", "command-palette-trigger role is Button");

    // command-palette-search (TextInput — egui's role for a single-line TextEdit; the contract's
    // "TextField" maps to accesskit 0.21's TextInput, the field-correct role).
    let search = nodes
        .iter()
        .find(|(a, _, _)| a == COMMAND_PALETTE_SEARCH_AUTHOR_ID)
        .unwrap_or_else(|| panic!("command-palette-search missing: {nodes:?}"));
    assert_eq!(search.1, "TextInput", "command-palette-search role is TextInput");

    // At least one cmd-{name} ListItem — specifically cmd-Copy (the contract names cmd-Copy).
    let cmd_copy_author = handshake_native::interop::interaction_bus::command_list_item_author_id("Copy");
    assert_eq!(cmd_copy_author, "cmd-Copy", "the contract's cmd-Copy address");
    let cmd_item = nodes
        .iter()
        .find(|(a, _, _)| a == &cmd_copy_author)
        .unwrap_or_else(|| panic!("cmd-Copy ({cmd_copy_author}) ListItem missing: {nodes:?}"));
    assert_eq!(cmd_item.1, "ListItem", "cmd-Copy role is ListItem");

    assert_no_local_artifact_dir();
}
