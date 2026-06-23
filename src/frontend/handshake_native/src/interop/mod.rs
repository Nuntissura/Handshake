//! Cross-surface interconnection substrate (WP-KERNEL-012 cluster E5 — melt-together).
//!
//! This subtree houses the app-wide primitives that let the four editor surfaces (code, rich-text,
//! graph, canvas) act as one tool rather than four stitched-on apps:
//!
//! - [`interaction_bus`] — the [`interaction_bus::InteractionBus`]: one [`interaction_bus::SharedSelection`]
//!   model, one mockable clipboard, and one cross-pane [`interaction_bus::CommandBus`] every pane shares
//!   through egui app data. It WRAPS the existing WP-011 substrate (`command_registry`, `event_bus`,
//!   `command_palette`) — it does not fork it (MT-031).
//! - [`adapters`] — the thin per-pane glue: the standard melt-together command set each pane registers,
//!   the selection/clipboard builders each pane uses, and the cross-pane command-surface AccessKit
//!   emitter (the `command-palette-trigger` / `command-palette-search` / `cmd-{id}` nodes the contract
//!   names) so a swarm agent can drive the shared command surface by stable id.
//!
//! Later E5 MTs (MT-032..MT-035: everything-is-a-block addressing, CKC embeds, code<->note refs,
//! unified undo, the Flight-Recorder event ledger) build ON this substrate rather than re-inventing
//! selection/clipboard/command state per surface.

pub mod adapters;
pub mod interaction_bus;

pub use interaction_bus::{
    command_list_item_author_id, default_keybind_for, interaction_bus_id, ClipboardPayload, CommandBus,
    CommandDescriptor, CommandHandler, EditorSurfaceKind, InteractionBus, SharedSelection, CMD_COPY,
    CMD_CUT, CMD_FIND, CMD_PASTE, CMD_SELECT_ALL, CMD_COMMAND_PALETTE,
    COMMAND_LIST_ITEM_AUTHOR_PREFIX, COMMAND_PALETTE_SEARCH_AUTHOR_ID,
    COMMAND_PALETTE_TRIGGER_AUTHOR_ID, INTERACTION_BUS_KEY,
};

pub use adapters::{
    register_standard_commands, surface_clipboard_payload, surface_command_ids, text_range_selection,
    CommandPaletteSurface,
};
