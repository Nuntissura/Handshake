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
    [CMD_COPY, CMD_CUT, CMD_PASTE, CMD_SELECT_ALL, CMD_FIND, CMD_COMMAND_PALETTE]
}

/// Register the standard melt-together command set into the shared bus for `surface`. Every pane calls
/// this once at construction so all four surfaces feed the ONE command bus (AC-4). The handlers are the
/// surface-agnostic ones (open palette, mark a no-op for the surface-specific edits the PANE wires to
/// its own buffer through the keybind path) — a pane that needs surface-specific Copy/Cut/Paste
/// behavior overrides the descriptor by re-registering the same id with its own handler AFTER this call.
///
/// The label is suffixed with the surface name so the palette listing disambiguates which surface a
/// command targets when several are registered, matching the React per-context command labels.
pub fn register_standard_commands(bus: &mut InteractionBus, surface: EditorSurfaceKind) {
    let surface_name = surface_label(surface);
    for &(id, name, base_label, keywords) in STANDARD_COMMANDS {
        // The CommandPalette opener is surface-agnostic and gets a real handler here; the edit commands
        // (Copy/Cut/Paste/SelectAll/Find) are registered so they are ADDRESSABLE + keybind-matchable on
        // every surface, and the pane re-registers the id with its buffer-specific handler.
        let handler: super::interaction_bus::CommandHandler = if id == CMD_COMMAND_PALETTE {
            Arc::new(|_ctx, bus| bus.open_command_palette())
        } else {
            // A safe default: a no-op until the pane re-registers with its buffer-specific handler. This
            // keeps the command present + addressable on every surface (AC-4) without inventing fake
            // edit behavior (the contract's no-mock rule).
            Arc::new(|_ctx, _bus| {})
        };
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

/// The id / short-name (→ `cmd-{name}` AccessKit author_id) / label / keywords for the standard command
/// set (the handler is chosen per-surface in [`register_standard_commands`]).
const STANDARD_COMMANDS: &[(&str, &str, &str, &[&str])] = &[
    (CMD_COPY, "Copy", "Copy", &["copy", "clipboard"]),
    (CMD_CUT, "Cut", "Cut", &["cut", "clipboard"]),
    (CMD_PASTE, "Paste", "Paste", &["paste", "clipboard"]),
    (CMD_SELECT_ALL, "SelectAll", "Select All", &["select", "all"]),
    (CMD_FIND, "Find", "Find", &["find", "search"]),
    (CMD_COMMAND_PALETTE, "CommandPalette", "Command Palette", &["command", "palette", "actions"]),
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
    SharedSelection::TextRange { pane_id, surface, start, end, text: text.into() }
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
            Self { last: StdMutex::new(None) }
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
            assert_eq!(bus.commands().len(), 6, "exactly the six standard commands for {surface:?}");
        }
    }

    /// AC-4: a graph/canvas surface copies a node ref as a `loom://` URI; a text surface copies plain
    /// text. Both go through the bus clipboard-write path.
    #[test]
    fn surface_clipboard_payload_maps_each_surface() {
        // Text surface -> PlainText.
        let text_sel = text_range_selection(pane("pane-code"), EditorSurfaceKind::Code, 0, 5, "hello");
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
        assert!(!copy_selection_to_clipboard(&mut bus, &SharedSelection::None, &mock));
    }

    /// The CommandPalette opener has a real handler (opens the palette); the edit commands default to a
    /// no-op (registered for addressability, the pane re-registers a buffer-specific handler).
    #[test]
    fn command_palette_opener_handler_opens_palette() {
        let ctx = egui::Context::default();
        let mut bus = InteractionBus::new();
        register_standard_commands(&mut bus, EditorSurfaceKind::Code);
        assert!(!bus.command_palette_open());
        assert!(bus.dispatch_command(&ctx, CMD_COMMAND_PALETTE));
        assert!(bus.command_palette_open(), "the standard CommandPalette handler opens the palette");
    }
}
