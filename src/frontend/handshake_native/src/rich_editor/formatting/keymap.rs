//! Keyboard shortcut bindings for the formatting commands (WP-KERNEL-012 MT-013).
//!
//! [`resolve_shortcut`] maps an `(egui::Modifiers, egui::Key)` pair to a
//! [`FormattingCommand`], matching the React `editor_keymap.ts` bindings + the Tiptap
//! StarterKit native chords the React `STARTERKIT_PROSE_CHORDS` list documents. The
//! input handler calls this BEFORE falling through to text insertion, so a chord like
//! `Ctrl+B` toggles bold instead of typing "b" (MT impl note 3).
//!
//! ## Binding table (contract KEYBOARD SHORTCUTS section, verbatim)
//!
//! | Chord                | Command                |
//! |----------------------|------------------------|
//! | Ctrl+B               | toggle_bold            |
//! | Ctrl+I               | toggle_italic          |
//! | Ctrl+U               | toggle_underline       |
//! | Ctrl+Shift+X         | toggle_strike          |
//! | Ctrl+E               | toggle_code            |
//! | Ctrl+Z               | undo                   |
//! | Ctrl+Shift+Z         | redo                   |
//! | Ctrl+Shift+7         | toggle_ordered_list    |
//! | Ctrl+Shift+8         | toggle_bullet_list     |
//! | Tab (in list)        | sink_list_item         |
//! | Shift+Tab (in list)  | lift_list_item         |
//! | Ctrl+Alt+1/2/3       | set_heading(1/2/3)     |
//! | Ctrl+Alt+0           | set_paragraph          |
//! | Ctrl+Shift+B         | set_blockquote         |
//! | Enter                | insert_paragraph_break |
//! | Backspace            | merge_backward (guard) |
//!
//! ## Platform-conflict note (red-team RISK-4 / MC-004)
//!
//! Three documented platform-specific chord conflicts (the contract's minimum control):
//! 1. **Ctrl+U** opens "view source" in some WEB browsers — N/A for the NATIVE app
//!    (no browser); egui owns the key. We consume it so it never bubbles to the OS.
//! 2. **Ctrl+Shift+7 / Ctrl+Shift+8** — on some keyboard layouts the shifted digit is
//!    a symbol (`&` / `*`); egui reports the PHYSICAL `Key::Num7`/`Num8`, so the
//!    binding keys off the egui `Key` (layout-independent) rather than the produced
//!    character.
//! 3. **Tab** is the focus-traversal key OS-wide; inside a list it indents instead, so
//!    the caller MUST consume it (return it from [`resolve_shortcut`]) only when the
//!    caret is in a list, else let Tab traverse focus. The list-context decision is the
//!    caller's (the dispatch refuses `sink_list_item` outside a list), but the keymap
//!    flags Tab/Shift+Tab as list-conditional via [`is_list_conditional`].
//!
//! The widget wraps dispatch in egui's `consume_key` for every resolved chord so the
//! key never double-fires (text insertion) or bubbles to the shell.

use egui::{Key, Modifiers};
use std::collections::{HashMap, HashSet};

use super::commands::FormattingCommand;
use crate::code_editor::keymap::KeyChord;
use crate::code_editor::keymap_settings::KeymapSettings;

/// Runtime rich-editor keymap. Defaults mirror [`resolve_shortcut`]; workspace-scoped `rich.*`
/// overrides replace the command's default chord and are then used by the mounted input handler.
/// Keeping this as owned editor state makes a settings GET/reopen rebind the running editor without
/// restarting the app.
#[derive(Clone, Debug)]
pub struct RichKeymap {
    bindings: HashMap<KeyChord, FormattingCommand>,
    overridden_commands: HashSet<&'static str>,
}

impl Default for RichKeymap {
    fn default() -> Self {
        let mut bindings = HashMap::new();
        for (chord, command) in default_bindings() {
            bindings.insert(chord, command);
        }
        Self {
            bindings,
            overridden_commands: HashSet::new(),
        }
    }
}

impl RichKeymap {
    /// Layer persisted rich-editor overrides over the built-in defaults. Each tuple contains the bare
    /// command id (without `rich.`) and a human-readable chord. Invalid ids/chords are returned to the
    /// caller and leave that command's working default intact; malformed persisted state never silently
    /// disables an editor action.
    pub fn from_overrides<'a>(
        overrides: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> (Self, Vec<String>) {
        let mut keymap = Self::default();
        let mut errors = Vec::new();
        for (command_id, chord_text) in overrides {
            let Some(command) = command_from_id(command_id) else {
                errors.push(format!("unknown rich-editor command '{command_id}'"));
                continue;
            };
            let chord = match KeymapSettings::chord_from_str(chord_text) {
                Ok(chord) => chord,
                Err(error) => {
                    errors.push(format!("{command_id}: {error}"));
                    continue;
                }
            };
            if let Some(existing) = keymap.bindings.get(&chord) {
                if existing.command_id() != command.command_id() {
                    errors.push(format!(
                        "{command_id}: chord '{chord_text}' is already bound to '{}'",
                        existing.command_id()
                    ));
                    continue;
                }
            }
            keymap
                .bindings
                .retain(|_, existing| existing.command_id() != command.command_id());
            keymap.overridden_commands.insert(command.command_id());
            keymap.bindings.insert(chord, command);
        }
        (keymap, errors)
    }

    /// Resolve one live egui key event through the current workspace-scoped map.
    pub fn resolve(&self, modifiers: &Modifiers, key: Key) -> Option<FormattingCommand> {
        self.bindings
            .get(&KeyChord::from_modifiers(key, modifiers))
            .cloned()
    }

    /// Whether a valid persisted row replaced this command's built-in chord. The input layer uses
    /// this for the legacy plain-edit fallbacks for Undo/Redo and boundary Backspace, ensuring those
    /// old defaults do not remain secretly active after the command is rebound.
    pub fn is_overridden(&self, command_id: &str) -> bool {
        self.overridden_commands.contains(command_id)
    }
}

fn default_bindings() -> Vec<(KeyChord, FormattingCommand)> {
    let defaults = [
        ("Mod+Alt+0", FormattingCommand::SetParagraph),
        ("Mod+Alt+1", FormattingCommand::SetHeading(1)),
        ("Mod+Alt+2", FormattingCommand::SetHeading(2)),
        ("Mod+Alt+3", FormattingCommand::SetHeading(3)),
        ("Mod+Shift+X", FormattingCommand::ToggleStrike),
        ("Mod+Shift+Z", FormattingCommand::Redo),
        ("Mod+Shift+7", FormattingCommand::ToggleOrderedList),
        ("Mod+Shift+8", FormattingCommand::ToggleBulletList),
        ("Mod+Shift+B", FormattingCommand::SetBlockquote),
        ("Mod+B", FormattingCommand::ToggleBold),
        ("Mod+I", FormattingCommand::ToggleItalic),
        ("Mod+U", FormattingCommand::ToggleUnderline),
        ("Mod+E", FormattingCommand::ToggleCode),
        ("Mod+Z", FormattingCommand::Undo),
        ("Shift+Tab", FormattingCommand::LiftListItem),
        ("Tab", FormattingCommand::SinkListItem),
        ("Enter", FormattingCommand::InsertParagraphBreak),
    ];
    defaults
        .into_iter()
        .map(|(chord, command)| {
            (
                KeymapSettings::chord_from_str(chord)
                    .expect("built-in rich-editor chord must remain parseable"),
                command,
            )
        })
        .collect()
}

fn command_from_id(command_id: &str) -> Option<FormattingCommand> {
    use FormattingCommand as F;
    Some(match command_id {
        "undo" => F::Undo,
        "redo" => F::Redo,
        "toggle_bold" => F::ToggleBold,
        "toggle_italic" => F::ToggleItalic,
        "toggle_underline" => F::ToggleUnderline,
        "toggle_strike" => F::ToggleStrike,
        "toggle_code" => F::ToggleCode,
        "set_paragraph" => F::SetParagraph,
        "set_heading_1" => F::SetHeading(1),
        "set_heading_2" => F::SetHeading(2),
        "set_heading_3" => F::SetHeading(3),
        "set_blockquote" => F::SetBlockquote,
        "set_code_block" => F::SetCodeBlock(None),
        "insert_horizontal_rule" => F::InsertHorizontalRule,
        "toggle_bullet_list" => F::ToggleBulletList,
        "toggle_ordered_list" => F::ToggleOrderedList,
        "toggle_task_list" => F::ToggleTaskList,
        "toggle_task_item_checked" => F::ToggleTaskItemChecked,
        "sink_list_item" => F::SinkListItem,
        "lift_list_item" => F::LiftListItem,
        "insert_table" => F::InsertTable { rows: 2, cols: 2 },
        "add_row_before" => F::AddRowBefore,
        "add_row_after" => F::AddRowAfter,
        "delete_row" => F::DeleteRow,
        "add_col_before" => F::AddColBefore,
        "add_col_after" => F::AddColAfter,
        "delete_col" => F::DeleteCol,
        "delete_table" => F::DeleteTable,
        "toggle_header_row" => F::ToggleHeaderRow,
        "insert_paragraph_break" => F::InsertParagraphBreak,
        "merge_backward" => F::MergeBackward,
        _ => return None,
    })
}

/// Resolve an egui key event `(modifiers, key)` to a [`FormattingCommand`], or `None`
/// when nothing is bound. Pure mapping (no state), so a test can assert the binding
/// table without a live egui context. The caller checks the result BEFORE treating the
/// key as text insertion (MT impl note 3).
///
/// `ctrl` is taken as `modifiers.command || modifiers.ctrl` so the binding is portable
/// (Cmd on macOS, Ctrl elsewhere — the same "Mod" convention the React keymap uses).
pub fn resolve_shortcut(modifiers: &Modifiers, key: Key) -> Option<FormattingCommand> {
    static DEFAULT_KEYMAP: std::sync::OnceLock<RichKeymap> = std::sync::OnceLock::new();
    DEFAULT_KEYMAP
        .get_or_init(RichKeymap::default)
        .resolve(modifiers, key)
}

/// True when a resolved command is "list-conditional": Tab/Shift+Tab indent/dedent only
/// make sense inside a list. The caller (input handler) should only CONSUME the Tab key
/// (preventing focus traversal) when the caret is in a list; otherwise it lets Tab fall
/// through to egui's focus navigation. `merge_backward` is similarly conditional (it is
/// a no-op when not at a block boundary), but Backspace is always consumed by the editor
/// (it is a text-editing key), so only Tab/Shift+Tab are flagged here.
pub fn is_list_conditional(cmd: &FormattingCommand) -> bool {
    matches!(
        cmd,
        FormattingCommand::SinkListItem | FormattingCommand::LiftListItem
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a modifiers value with ctrl set (command mirrors ctrl for portability).
    fn ctrl() -> Modifiers {
        Modifiers {
            ctrl: !cfg!(target_os = "macos"),
            command: true,
            mac_cmd: cfg!(target_os = "macos"),
            ..Default::default()
        }
    }
    fn ctrl_shift() -> Modifiers {
        Modifiers {
            ctrl: !cfg!(target_os = "macos"),
            command: true,
            mac_cmd: cfg!(target_os = "macos"),
            shift: true,
            ..Default::default()
        }
    }
    fn ctrl_alt() -> Modifiers {
        Modifiers {
            ctrl: !cfg!(target_os = "macos"),
            command: true,
            mac_cmd: cfg!(target_os = "macos"),
            alt: true,
            ..Default::default()
        }
    }
    fn none() -> Modifiers {
        Modifiers::default()
    }
    fn shift() -> Modifiers {
        Modifiers {
            shift: true,
            ..Default::default()
        }
    }

    #[test]
    fn ctrl_b_is_bold() {
        assert_eq!(
            resolve_shortcut(&ctrl(), Key::B),
            Some(FormattingCommand::ToggleBold)
        );
    }

    #[test]
    fn ctrl_i_u_e_marks() {
        assert_eq!(
            resolve_shortcut(&ctrl(), Key::I),
            Some(FormattingCommand::ToggleItalic)
        );
        assert_eq!(
            resolve_shortcut(&ctrl(), Key::U),
            Some(FormattingCommand::ToggleUnderline)
        );
        assert_eq!(
            resolve_shortcut(&ctrl(), Key::E),
            Some(FormattingCommand::ToggleCode)
        );
    }

    #[test]
    fn ctrl_shift_x_is_strike_not_bold() {
        // Shift disambiguates: Ctrl+Shift+X is strike; plain Ctrl+B stays bold.
        assert_eq!(
            resolve_shortcut(&ctrl_shift(), Key::X),
            Some(FormattingCommand::ToggleStrike)
        );
        assert_eq!(
            resolve_shortcut(&ctrl_shift(), Key::B),
            Some(FormattingCommand::SetBlockquote),
            "Ctrl+Shift+B is blockquote, NOT bold"
        );
    }

    #[test]
    fn undo_redo_chords() {
        assert_eq!(
            resolve_shortcut(&ctrl(), Key::Z),
            Some(FormattingCommand::Undo)
        );
        assert_eq!(
            resolve_shortcut(&ctrl_shift(), Key::Z),
            Some(FormattingCommand::Redo)
        );
    }

    #[test]
    fn list_chords_use_physical_digit_key() {
        // RISK-4 / MC-004: bind off the egui Key (physical), not the shifted symbol.
        assert_eq!(
            resolve_shortcut(&ctrl_shift(), Key::Num7),
            Some(FormattingCommand::ToggleOrderedList)
        );
        assert_eq!(
            resolve_shortcut(&ctrl_shift(), Key::Num8),
            Some(FormattingCommand::ToggleBulletList)
        );
    }

    #[test]
    fn heading_and_paragraph_chords() {
        assert_eq!(
            resolve_shortcut(&ctrl_alt(), Key::Num0),
            Some(FormattingCommand::SetParagraph)
        );
        assert_eq!(
            resolve_shortcut(&ctrl_alt(), Key::Num1),
            Some(FormattingCommand::SetHeading(1))
        );
        assert_eq!(
            resolve_shortcut(&ctrl_alt(), Key::Num2),
            Some(FormattingCommand::SetHeading(2))
        );
        assert_eq!(
            resolve_shortcut(&ctrl_alt(), Key::Num3),
            Some(FormattingCommand::SetHeading(3))
        );
    }

    #[test]
    fn tab_and_shift_tab_are_list_conditional() {
        let sink = resolve_shortcut(&none(), Key::Tab).unwrap();
        let lift = resolve_shortcut(&shift(), Key::Tab).unwrap();
        assert_eq!(sink, FormattingCommand::SinkListItem);
        assert_eq!(lift, FormattingCommand::LiftListItem);
        assert!(is_list_conditional(&sink));
        assert!(is_list_conditional(&lift));
        assert!(!is_list_conditional(&FormattingCommand::ToggleBold));
    }

    #[test]
    fn enter_is_split_backspace_is_not_claimed() {
        // Enter is a formatting chord (split the block); Backspace is NOT claimed by the
        // keymap (the input handler's text decode owns it and routes offset-0 backspace
        // to merge_backward — a single key path).
        assert_eq!(
            resolve_shortcut(&none(), Key::Enter),
            Some(FormattingCommand::InsertParagraphBreak)
        );
        assert_eq!(resolve_shortcut(&none(), Key::Backspace), None);
    }

    #[test]
    fn unbound_keys_return_none() {
        assert_eq!(resolve_shortcut(&none(), Key::A), None);
        assert_eq!(resolve_shortcut(&ctrl(), Key::Q), None);
    }

    #[test]
    fn persisted_override_replaces_default_and_can_bind_toolbar_only_command() {
        let (keymap, errors) = RichKeymap::from_overrides([
            ("toggle_bold", "Mod+Alt+B"),
            ("insert_horizontal_rule", "Mod+Alt+R"),
        ]);
        assert!(errors.is_empty(), "valid overrides: {errors:?}");
        assert_eq!(keymap.resolve(&ctrl(), Key::B), None);
        assert_eq!(
            keymap.resolve(&ctrl_alt(), Key::B),
            Some(FormattingCommand::ToggleBold)
        );
        assert_eq!(
            keymap.resolve(&ctrl_alt(), Key::R),
            Some(FormattingCommand::InsertHorizontalRule)
        );
    }

    #[test]
    fn invalid_persisted_override_keeps_working_default_and_returns_error() {
        let (keymap, errors) =
            RichKeymap::from_overrides([("toggle_bold", "Mod+DefinitelyNotAKey")]);
        assert_eq!(errors.len(), 1);
        assert_eq!(
            keymap.resolve(&ctrl(), Key::B),
            Some(FormattingCommand::ToggleBold),
            "bad persisted state never silently disables the default"
        );
    }

    #[test]
    fn valid_override_records_that_legacy_plain_default_must_be_suppressed() {
        let (keymap, errors) = RichKeymap::from_overrides([
            ("undo", "Mod+Alt+Z"),
            ("merge_backward", "Mod+Alt+Backspace"),
        ]);
        assert!(errors.is_empty());
        assert!(keymap.is_overridden("undo"));
        assert!(keymap.is_overridden("merge_backward"));
        assert!(!keymap.is_overridden("redo"));
        assert_eq!(keymap.resolve(&ctrl(), Key::Z), None);
    }

    #[test]
    fn chord_collision_is_rejected_without_disabling_either_working_default() {
        let (keymap, errors) = RichKeymap::from_overrides([("toggle_italic", "Mod+B")]);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("already bound to 'toggle_bold'"));
        assert_eq!(
            keymap.resolve(&ctrl(), Key::B),
            Some(FormattingCommand::ToggleBold)
        );
        assert_eq!(
            keymap.resolve(&ctrl(), Key::I),
            Some(FormattingCommand::ToggleItalic)
        );
        assert!(!keymap.is_overridden("toggle_italic"));
    }
}
