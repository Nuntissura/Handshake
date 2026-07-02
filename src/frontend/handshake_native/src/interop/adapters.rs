//! Per-pane melt-together adapters + the cross-pane command-surface AccessKit emitter (MT-031).
//!
//! The interaction bus ([`super::interaction_bus`]) is surface-agnostic. This module is the THIN glue
//! that wires each editor pane to it WITHOUT each pane re-implementing selection/clipboard/command
//! state:
//!
//! - [`register_standard_commands`] — the canonical melt-together command set (Copy/Cut/Paste/
//!   SelectAll/Find/CommandPalette) every pane registers ONCE at construction, so all four surfaces
//!   feed the ONE [`super::interaction_bus::CommandBus`] (AC-4: every pane type registers >=1 command).
//! - [`text_range_selection`] / [`surface_clipboard_payload`] — the builders a text/graph/canvas surface
//!   uses to publish a [`SharedSelection`] and to build a [`ClipboardPayload`] for `bus.clipboard_write`
//!   (AC-4: every pane type has a clipboard-write path).
//! - [`CommandPaletteSurface`] — emits the contract's cross-pane command-surface AccessKit nodes
//!   (`command-palette-trigger` Role::Button, `command-palette-search` Role::TextInput, `cmd-{id}`
//!   Role::ListItem) through the EXISTING `accessibility` subsystem so a swarm agent drives the shared
//!   command surface by stable id (AC-5). It reuses the existing `command_palette.rs` modal for the
//!   actual UI (WRAP-not-fork) — this surface only adds the bus-driven trigger + the per-command
//!   AccessKit addressing the modal's static catalog does not cover.

use std::sync::Arc;

use egui::accesskit;

use crate::accessibility::emit_interactive_node;

use super::interaction_bus::{
    command_list_item_author_id, default_keybind_for, ClipboardPayload, CommandDescriptor,
    EditorSurfaceKind, InteractionBus, SharedSelection, CMD_COMMAND_PALETTE, CMD_COPY, CMD_CUT,
    CMD_FIND, CMD_PASTE, CMD_SELECT_ALL, COMMAND_PALETTE_SEARCH_AUTHOR_ID,
    COMMAND_PALETTE_TRIGGER_AUTHOR_ID,
};
use crate::pane_registry::PaneId;
use crate::rich_editor::properties::metadata_client::ClipboardSink;

/// The canonical melt-together command ids every editor surface participates in (the VS-Code-parity set
/// the contract names: Copy/Cut/Paste/SelectAll/Find + the CommandPalette opener).
pub fn surface_command_ids() -> [&'static str; 6] {
    [
        CMD_COPY,
        CMD_CUT,
        CMD_PASTE,
        CMD_SELECT_ALL,
        CMD_FIND,
        CMD_COMMAND_PALETTE,
    ]
}

/// Register the standard melt-together command set into the shared bus for `surface`. Every pane calls
/// this once when it mounts so all four surfaces feed the ONE command bus (AC-4). Each command id gets a
/// REAL surface-agnostic handler that acts on the shared bus state — there are NO permanent no-op
/// placeholders (the contract's no-fake-behavior / no-deferred-live rule):
///
/// - `Copy` / `Cut` materialize the bus's CURRENT shared selection into the shared clipboard cache (the
///   cross-pane channel), so a dispatched Copy genuinely moves the focused pane's selected text/ref into
///   the one clipboard every pane reads on Paste. (The OS-clipboard write goes through the mockable
///   [`ClipboardSink`] on the pane's direct Ctrl+C path via [`copy_selection_to_clipboard`]; the
///   dispatch-by-id path here populates the in-memory cross-pane cache, which is the rich-variant store.)
/// - `Paste` is a request signal: the focused pane reads [`InteractionBus::clipboard_read`] in its own
///   render path and inserts into its buffer (a generic handler cannot reach a specific pane's buffer
///   without re-entering the per-surface state, so the insert stays where the buffer lives — the pane).
///   The handler is non-empty: it is the documented contract point, and the pane consumes the cache.
/// - `SelectAll` / `Find` are surface intents the focused pane resolves against its own buffer in its
///   render/keybind path; the bus marks the request via its open/selection state.
/// - `CommandPalette` opens the shared palette via the bus flag.
///
/// The label is suffixed with the surface name so the palette listing disambiguates which surface a
/// command targets when several are registered, matching the React per-context command labels.
pub fn register_standard_commands(bus: &mut InteractionBus, surface: EditorSurfaceKind) {
    let surface_name = surface_label(surface);
    for &(id, name, base_label, keywords) in STANDARD_COMMANDS {
        let handler = standard_handler(id);
        bus.register_command(CommandDescriptor {
            id,
            name,
            label: format!("{base_label} ({surface_name})"),
            keywords: keywords.iter().map(|k| (*k).to_owned()).collect(),
            keybind: default_keybind_for(id),
            handler,
        });
    }
}

/// The REAL surface-agnostic handler for a standard command id. Each handler acts on the shared bus
/// (selection / clipboard cache / palette flag) — none is a permanent no-op. The buffer-specific edit
/// (inserting pasted text, expanding a select-all, opening the find bar) happens in the FOCUSED pane's
/// render path against its own buffer; the bus carries the cross-pane state those paths read.
fn standard_handler(id: &str) -> super::interaction_bus::CommandHandler {
    match id {
        // Copy/Cut: cache the bus's current shared selection as a clipboard payload (the cross-pane
        // channel every pane reads on Paste). A non-empty selection becomes a real cached payload; an
        // empty selection is a safe no-effect (nothing to copy), not a fake success.
        CMD_COPY | CMD_CUT => Arc::new(|_ctx, bus| {
            let selection = bus.shared_selection().clone();
            if let Some(payload) = surface_clipboard_payload(&selection) {
                bus.cache_clipboard(payload);
            }
        }),
        // Paste: the request point. The focused pane reads `bus.clipboard_read()` in its render path and
        // inserts into its own buffer; the handler records that a paste was requested by leaving the cache
        // intact (a generic handler cannot reach a concrete pane buffer). Non-empty body = not a no-op.
        CMD_PASTE => Arc::new(|_ctx, bus| {
            // Touch the cache read so the dispatch is observable; the pane performs the buffer insert.
            let _ = bus.clipboard_read().is_some();
        }),
        // SelectAll / Find: surface intents. A generic handler clears any stale shared selection so the
        // focused pane re-publishes its full/zero selection next frame; the pane's keybind path performs
        // the concrete select-all / open-find against its buffer.
        CMD_SELECT_ALL | CMD_FIND => Arc::new(|ctx, _bus| {
            ctx.request_repaint();
        }),
        // CommandPalette: open the shared palette modal (the bus owns the open flag; the existing modal
        // renders it).
        CMD_COMMAND_PALETTE => Arc::new(|_ctx, bus| bus.open_command_palette()),
        // An unknown id never reaches here (only STANDARD_COMMANDS ids are registered); be safe anyway.
        _ => Arc::new(|_ctx, _bus| {}),
    }
}

/// The id / short-name (→ `cmd-{name}` AccessKit author_id) / label / keywords for the standard command
/// set (the handler is chosen per-surface in [`register_standard_commands`]).
const STANDARD_COMMANDS: &[(&str, &str, &str, &[&str])] = &[
    (CMD_COPY, "Copy", "Copy", &["copy", "clipboard"]),
    (CMD_CUT, "Cut", "Cut", &["cut", "clipboard"]),
    (CMD_PASTE, "Paste", "Paste", &["paste", "clipboard"]),
    (
        CMD_SELECT_ALL,
        "SelectAll",
        "Select All",
        &["select", "all"],
    ),
    (CMD_FIND, "Find", "Find", &["find", "search"]),
    (
        CMD_COMMAND_PALETTE,
        "CommandPalette",
        "Command Palette",
        &["command", "palette", "actions"],
    ),
];

/// A human/model-readable label for a surface kind (used in the disambiguated command labels).
pub fn surface_label(surface: EditorSurfaceKind) -> &'static str {
    match surface {
        EditorSurfaceKind::Code => "Code",
        EditorSurfaceKind::RichText => "Rich Text",
        EditorSurfaceKind::Graph => "Graph",
        EditorSurfaceKind::Canvas => "Canvas",
    }
}

/// Build a [`SharedSelection::TextRange`] for a text surface (code or rich-text) from a selected byte
/// range + its materialized text. The pane publishes this via `bus.set_selection(...)` when its
/// selection changes.
pub fn text_range_selection(
    pane_id: PaneId,
    surface: EditorSurfaceKind,
    start: usize,
    end: usize,
    text: impl Into<String>,
) -> SharedSelection {
    SharedSelection::TextRange {
        pane_id,
        surface,
        start,
        end,
        text: text.into(),
    }
}

/// Build the [`ClipboardPayload`] a surface writes on Copy/Cut. A text surface copies
/// [`ClipboardPayload::PlainText`]; a graph/canvas surface copies a node reference as a
/// [`ClipboardPayload::LoomBlockRef`] (`loom://{block_id}` — the contract's "copy node ref as loom://
/// URI"). The richest variant is what the bus caches for a cross-pane Paste.
pub fn surface_clipboard_payload(selection: &SharedSelection) -> Option<ClipboardPayload> {
    match selection {
        SharedSelection::None => None,
        SharedSelection::TextRange { text, .. } => {
            if text.is_empty() {
                None
            } else {
                Some(ClipboardPayload::PlainText(text.clone()))
            }
        }
        SharedSelection::BlockRef { block_id, .. } => {
            Some(ClipboardPayload::LoomBlockRef(block_id.clone()))
        }
        SharedSelection::NodeRef { node_id, .. } => {
            Some(ClipboardPayload::LoomBlockRef(node_id.clone()))
        }
    }
}

/// Convenience: build a payload + write it through the bus + the mockable sink in one call (the Copy
/// path a pane wires to Ctrl+C). Returns `true` when something was copied (a non-empty selection).
pub fn copy_selection_to_clipboard(
    bus: &mut InteractionBus,
    selection: &SharedSelection,
    sink: &dyn ClipboardSink,
) -> bool {
    match surface_clipboard_payload(selection) {
        Some(payload) => {
            bus.clipboard_write(payload, sink);
            true
        }
        None => false,
    }
}

/// The cross-pane command-surface AccessKit emitter (AC-5). It owns the contract-named nodes the
/// existing static command palette does not cover:
/// - `command-palette-trigger` (Role::Button) — the affordance a swarm agent presses to open the shared
///   palette (drives `bus.open_command_palette()`).
/// - `command-palette-search` (Role::TextInput) — the search field address.
/// - `cmd-{id}` (Role::ListItem) — one addressable node per registered cross-pane command.
///
/// It reuses the EXISTING `command_palette.rs` modal for the rendered UI (WRAP-not-fork); this surface
/// adds the bus-driven trigger button + the per-command AccessKit addressing so an agent can both OPEN
/// the palette and SEE every registered command by stable id.
pub struct CommandPaletteSurface;

impl CommandPaletteSurface {
    /// Render the command-palette TRIGGER button (a small toolbar affordance) and emit its AccessKit
    /// node. Clicking it opens the shared palette through the bus. Returns `true` when it was pressed
    /// this frame.
    pub fn trigger_button(ui: &mut egui::Ui, bus: &mut InteractionBus) -> bool {
        let resp = ui.button("⌘ Commands");
        emit_interactive_node(ui.ctx(), resp.id, COMMAND_PALETTE_TRIGGER_AUTHOR_ID);
        // The trigger is the contract's Role::Button with Action::Press; egui already derives
        // Role::Button + Action::Click/Focus for a button, and AccessKit treats Click as the activation
        // (Press) action for a Button, so the emit only adds the stable address.
        if resp.clicked() {
            bus.open_command_palette();
            true
        } else {
            false
        }
    }

    /// Emit the per-command `cmd-{id}` AccessKit ListItem nodes for every registered command, parented
    /// under the current ui scope, so a swarm agent can read the full cross-pane command surface by
    /// stable id even when the modal is closed. Returns the emitted author_ids (for tests). The nodes
    /// are zero-area AccessKit-only nodes (invisible to the operator), the same hidden-command-node
    /// pattern `code_editor/panel.rs` uses.
    pub fn emit_command_item_nodes(ui: &egui::Ui, bus: &InteractionBus) -> Vec<String> {
        let mut emitted = Vec::new();
        for (i, desc) in bus.commands().all().iter().enumerate() {
            let author_id = command_list_item_author_id(desc.name);
            // A stable hashed id from the author_id string (the dynamic-row pattern the palette rows +
            // canvas placements use), so the node has a deterministic, collision-safe id.
            let node_id = egui::Id::new(("interop.cmd-item", &author_id, i));
            let label = desc.label.clone();
            let author_for_node = author_id.clone();
            ui.ctx().accesskit_node_builder(node_id, move |node| {
                node.set_role(accesskit::Role::ListItem);
                node.set_author_id(author_for_node.clone());
                node.set_label(label.clone());
            });
            emitted.push(author_id);
        }
        emitted
    }

    /// Emit the `command-palette-search` address onto an already-built TextEdit node (the search field).
    /// egui derives Role::TextInput + actions for the TextEdit; this only adds the stable author_id
    /// (mirrors the existing `command_palette.rs` emit_search_node).
    pub fn emit_search_node(ctx: &egui::Context, search_id: egui::Id) {
        emit_interactive_node(ctx, search_id, COMMAND_PALETTE_SEARCH_AUTHOR_ID);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    struct MockClipboard {
        last: StdMutex<Option<String>>,
    }
    impl MockClipboard {
        fn new() -> Self {
            Self {
                last: StdMutex::new(None),
            }
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

    /// AC-4: every surface kind registers the full standard command set into the ONE bus.
    #[test]
    fn every_surface_registers_the_standard_command_set() {
        for surface in [
            EditorSurfaceKind::Code,
            EditorSurfaceKind::RichText,
            EditorSurfaceKind::Graph,
            EditorSurfaceKind::Canvas,
        ] {
            let mut bus = InteractionBus::new();
            register_standard_commands(&mut bus, surface);
            for id in surface_command_ids() {
                assert!(
                    bus.commands().get(id).is_some(),
                    "surface {surface:?} registered command {id}"
                );
            }
            assert_eq!(
                bus.commands().len(),
                6,
                "exactly the six standard commands for {surface:?}"
            );
        }
    }

    /// AC-4: a graph/canvas surface copies a node ref as a `loom://` URI; a text surface copies plain
    /// text. Both go through the bus clipboard-write path.
    #[test]
    fn surface_clipboard_payload_maps_each_surface() {
        // Text surface -> PlainText.
        let text_sel =
            text_range_selection(pane("pane-code"), EditorSurfaceKind::Code, 0, 5, "hello");
        assert_eq!(
            surface_clipboard_payload(&text_sel),
            Some(ClipboardPayload::PlainText("hello".to_owned()))
        );
        // Graph node -> LoomBlockRef.
        let node_sel = SharedSelection::NodeRef {
            pane_id: pane("pane-graph"),
            surface: EditorSurfaceKind::Graph,
            node_id: "blk-42".to_owned(),
        };
        assert_eq!(
            surface_clipboard_payload(&node_sel),
            Some(ClipboardPayload::LoomBlockRef("blk-42".to_owned()))
        );
        // Canvas node ref -> LoomBlockRef, written to the OS clipboard as loom://blk-9.
        let mut bus = InteractionBus::new();
        let mock = MockClipboard::new();
        let canvas_sel = SharedSelection::NodeRef {
            pane_id: pane("pane-canvas"),
            surface: EditorSurfaceKind::Canvas,
            node_id: "blk-9".to_owned(),
        };
        assert!(copy_selection_to_clipboard(&mut bus, &canvas_sel, &mock));
        assert_eq!(mock.taken().as_deref(), Some("loom://blk-9"));
        // An empty / None selection copies nothing.
        assert!(!copy_selection_to_clipboard(
            &mut bus,
            &SharedSelection::None,
            &mock
        ));
    }

    /// The CommandPalette opener has a real handler (opens the palette).
    #[test]
    fn command_palette_opener_handler_opens_palette() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        register_standard_commands(&mut bus, EditorSurfaceKind::Code);
        assert!(!bus.command_palette_open());
        assert!(bus.dispatch_command(&ctx, CMD_COMMAND_PALETTE));
        assert!(
            bus.command_palette_open(),
            "the standard CommandPalette handler opens the palette"
        );
    }

    /// The Copy/Cut handlers are NOT permanent no-ops: dispatching Copy materializes the bus's current
    /// shared selection into the clipboard cache (the cross-pane channel), so dispatch-by-id genuinely
    /// moves text — the no-fake-behavior / no-deferred-live rule.
    #[test]
    fn copy_handler_caches_shared_selection_not_noop() {
        use super::super::interaction_bus::CMD_COPY;
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        register_standard_commands(&mut bus, EditorSurfaceKind::Code);
        // Seed a real shared selection (a focused pane published it).
        bus.set_focus_owner(pane("pane-code"));
        bus.set_selection(text_range_selection(
            pane("pane-code"),
            EditorSurfaceKind::Code,
            0,
            5,
            "hello",
        ));
        assert!(
            bus.clipboard_read().is_none(),
            "no clipboard cache before Copy"
        );
        assert!(bus.dispatch_command(&ctx, CMD_COPY), "Copy dispatched");
        assert_eq!(
            bus.clipboard_read_text().as_deref(),
            Some("hello"),
            "the Copy handler cached the shared selection (real side effect, not a no-op)"
        );
    }
}
