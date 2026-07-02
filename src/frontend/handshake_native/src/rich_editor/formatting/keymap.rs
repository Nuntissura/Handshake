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

use super::commands::FormattingCommand;

/// Resolve an egui key event `(modifiers, key)` to a [`FormattingCommand`], or `None`
/// when nothing is bound. Pure mapping (no state), so a test can assert the binding
/// table without a live egui context. The caller checks the result BEFORE treating the
/// key as text insertion (MT impl note 3).
///
/// `ctrl` is taken as `modifiers.command || modifiers.ctrl` so the binding is portable
/// (Cmd on macOS, Ctrl elsewhere — the same "Mod" convention the React keymap uses).
pub fn resolve_shortcut(modifiers: &Modifiers, key: Key) -> Option<FormattingCommand> {
    let ctrl = modifiers.command || modifiers.ctrl;
    let shift = modifiers.shift;
    let alt = modifiers.alt;

    match key {
        // --- Ctrl+Alt+digit: block-kind set (checked BEFORE plain Ctrl chords so the
        //     alt modifier disambiguates). ---
        Key::Num0 if ctrl && alt => Some(FormattingCommand::SetParagraph),
        Key::Num1 if ctrl && alt => Some(FormattingCommand::SetHeading(1)),
        Key::Num2 if ctrl && alt => Some(FormattingCommand::SetHeading(2)),
        Key::Num3 if ctrl && alt => Some(FormattingCommand::SetHeading(3)),

        // --- Ctrl+Shift chords (checked before plain Ctrl so shift disambiguates). ---
        Key::X if ctrl && shift => Some(FormattingCommand::ToggleStrike),
        Key::Z if ctrl && shift => Some(FormattingCommand::Redo),
        Key::Num7 if ctrl && shift => Some(FormattingCommand::ToggleOrderedList),
        Key::Num8 if ctrl && shift => Some(FormattingCommand::ToggleBulletList),
        Key::B if ctrl && shift => Some(FormattingCommand::SetBlockquote),

        // --- plain Ctrl chords (no shift/alt) ---
        Key::B if ctrl && !shift && !alt => Some(FormattingCommand::ToggleBold),
        Key::I if ctrl && !shift && !alt => Some(FormattingCommand::ToggleItalic),
        Key::U if ctrl && !shift && !alt => Some(FormattingCommand::ToggleUnderline),
        Key::E if ctrl && !shift && !alt => Some(FormattingCommand::ToggleCode),
        Key::Z if ctrl && !shift && !alt => Some(FormattingCommand::Undo),

        // --- list indent/dedent (Tab / Shift+Tab) — list-conditional (caller guards
        //     the list context; dispatch refuses outside a list). ---
        Key::Tab if shift && !ctrl && !alt => Some(FormattingCommand::LiftListItem),
        Key::Tab if !shift && !ctrl && !alt => Some(FormattingCommand::SinkListItem),

        // --- structural editing (MT-013 scope expansion) ---
        // Enter is claimed by the formatting layer (split the block). Backspace is NOT
        // claimed here: it stays a TEXT-EDITING key in the input handler's plain decode,
        // which now routes a backspace at block offset 0 to merge_backward (so a single
        // key path owns backspace — see `input_handler::delete`). Claiming Backspace in
        // BOTH the keymap and the text decode would double-fire (merge + char delete).
        Key::Enter if !ctrl && !shift && !alt => Some(FormattingCommand::InsertParagraphBreak),

        _ => None,
    }
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
            ctrl: true,
            command: true,
            ..Default::default()
        }
    }
    fn ctrl_shift() -> Modifiers {
        Modifiers {
            ctrl: true,
            command: true,
            shift: true,
            ..Default::default()
        }
    }
    fn ctrl_alt() -> Modifiers {
        Modifiers {
            ctrl: true,
            command: true,
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
}
