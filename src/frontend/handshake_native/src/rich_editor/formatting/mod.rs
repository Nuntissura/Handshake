//! Block- and inline-level formatting commands, the editor toolbar, and the keyboard
//! shortcut bindings for the native rich-text editor (WP-KERNEL-012 MT-013).
//!
//! This is the COMMAND + CHROME layer of the E2 rich-text cluster, built on top of the
//! MT-011 document model ([`crate::rich_editor::document_model`]) and the MT-012
//! renderer ([`crate::rich_editor::renderer`]). It delivers, at command-for-command
//! parity with the React `editor_commands.ts` catalog (categories `format | block |
//! list | table | tableEdit | history`):
//!
//! - [`commands`] — typed [`commands::FormattingCommand`]s + the [`commands::dispatch`]
//!   entry every surface calls. Each command builds an MT-011 `Transaction`, applies it
//!   atomically, pushes the receipt onto the `UndoManager`, and moves the caret.
//! - [`toolbar`] — the [`toolbar::EditorToolbar`] egui widget: a horizontal row of
//!   glyph buttons grouped by category, with active-state highlight, AccessKit
//!   `toolbar-btn-{id}` author_ids, and an overflow popup.
//! - [`keymap`] — [`keymap::resolve_shortcut`] mapping `(Modifiers, Key)` to a
//!   `FormattingCommand`, matching the React `editor_keymap.ts` bindings.
//!
//! All command activation runs STANDALONE on the local editor state (the WP-011
//! command_registry/event_bus host Sender is not wired into the editor until E11/MT-069
//! — bus routing is additive, never the only path; COMMAND DISPATCH REALITY gate).

pub mod commands;
pub mod keymap;
pub mod toolbar;

pub use commands::{dispatch, is_mark_active, CommandContext, CommandError, FormattingCommand};
pub use keymap::{is_list_conditional, resolve_shortcut};
pub use toolbar::{
    all_toolbar_commands, is_command_active, toolbar_button_author_id, EditorToolbar,
    TOOLBAR_BTN_AUTHOR_PREFIX,
};
