//! Monaco / VS Code parity keymap for the native code editor (WP-KERNEL-012 — E1 MT-010).
//!
//! This module is the SINGLE canonical answer to "what does this key do in the code editor". It ports
//! `app/src/lib/editor/editor_keymap.ts` to Rust: the [`KeyBinding`] table (the React `KeyBinding`
//! interface + `EDITOR_KEY_BINDINGS`), [`Keymap::resolve`] (the React `resolveShortcut`), and the
//! chord model (the React `chordFromEvent`). The React editor's keymap was a small prose-editor table;
//! the native code editor needs the FULL Monaco caret-movement + editing + navigation command set, so
//! [`CodeEditorAction`] is the complete VS-Code-parity command enum and [`Keymap::default_vscode`]
//! returns the canonical binding table.
//!
//! ## One dispatch point
//!
//! Before MT-010, each E1 microtask (MT-003 Ctrl+D, MT-004 Ctrl+F/H, MT-005 Ctrl+Shift+[/], MT-006
//! Ctrl+G, MT-008 completion-popup keys) wired its own `egui::Event::Key` match arm directly in
//! `panel.rs::process_cursor_input`. MT-010 consolidates them: `panel.rs::process_keymap` reads input
//! events, calls [`Keymap::resolve`] (single lookup table), and dispatches the resolved
//! [`CodeEditorAction`] to the existing per-feature handler methods. The keymap is the one place a
//! binding is declared, so it is also the one place an operator override or a swarm agent's
//! command-node activation feeds into — no scattered key handling.
//!
//! ## OS-agnostic Mod key
//!
//! The canonical binding table uses [`mod_key`] (Ctrl on Windows/Linux, Command on macOS) for the
//! "Mod" chords, decided at compile time via `cfg!(target_os = "macos")` (RISK-004 — a native
//! platform-specific binary is expected; this is not a runtime-overridable matrix, which is acceptable
//! per the MT red-team). Operator overrides loaded from the settings file can still rebind any action.

use std::collections::HashMap;

/// One key chord: a single [`egui::Key`] plus the modifier flags that must be held. Two chords are
/// equal iff the key and the (CTRL/ALT/SHIFT/MAC_CMD) modifier flags match. egui's
/// [`egui::Modifiers`] also carries a `command` convenience flag that mirrors CTRL on
/// Windows/Linux and MAC_CMD on macOS; we normalize on the explicit ctrl/alt/shift/mac_cmd booleans so
/// a chord compares deterministically regardless of how the event was constructed (the same
/// normalization the React `chordFromEvent` did by collapsing ctrlKey/metaKey to "Mod").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord {
    /// The physical key (letter, digit, function key, arrow, etc.).
    pub key: egui::Key,
    /// CTRL held.
    pub ctrl: bool,
    /// ALT held.
    pub alt: bool,
    /// SHIFT held.
    pub shift: bool,
    /// macOS Command held (the MAC_CMD modifier). On Windows/Linux this is always `false`; the "Mod"
    /// chords resolve to `ctrl` there. Tracked separately so a binding authored with [`mod_key`] on
    /// macOS does not accidentally match a Ctrl chord (VS Code keeps Cmd and Ctrl distinct on macOS).
    pub mac_cmd: bool,
}

impl KeyChord {
    /// A chord with no modifiers (e.g. a bare `F12`, `Escape`, `Tab`).
    pub const fn plain(key: egui::Key) -> Self {
        Self { key, ctrl: false, alt: false, shift: false, mac_cmd: false }
    }

    /// A chord with explicit modifier flags.
    pub const fn new(key: egui::Key, ctrl: bool, alt: bool, shift: bool, mac_cmd: bool) -> Self {
        Self { key, ctrl, alt, shift, mac_cmd }
    }

    /// Build a chord from an [`egui::Key`] plus an [`egui::Modifiers`], normalizing onto the explicit
    /// ctrl/alt/shift/mac_cmd flags. This is how `process_keymap` turns a live `egui::Event::Key` into a
    /// chord for [`Keymap::resolve`] (the equivalent of the React `chordFromEvent`).
    pub fn from_modifiers(key: egui::Key, modifiers: &egui::Modifiers) -> Self {
        Self {
            key,
            ctrl: modifiers.ctrl,
            alt: modifiers.alt,
            shift: modifiers.shift,
            mac_cmd: modifiers.mac_cmd,
        }
    }

    /// The [`egui::Modifiers`] this chord requires (so a caller can compare against a live modifier
    /// state or construct a synthetic event for tests).
    pub fn modifiers(&self) -> egui::Modifiers {
        egui::Modifiers {
            alt: self.alt,
            ctrl: self.ctrl,
            shift: self.shift,
            mac_cmd: self.mac_cmd,
            // `command` is the OS-normalized convenience flag egui sets; mirror ctrl on non-mac and
            // mac_cmd on mac so a chord we hand back to egui round-trips.
            command: if cfg!(target_os = "macos") { self.mac_cmd } else { self.ctrl },
        }
    }
}

/// The "Mod" modifier set: Command on macOS, Ctrl elsewhere. The canonical binding table uses this so
/// the SAME table is correct on every platform (RISK-004). A `const fn` so the table can be built in a
/// `const`/static-friendly way and so the platform branch is resolved at compile time.
pub const fn mod_is_ctrl() -> bool {
    !cfg!(target_os = "macos")
}

/// Build a chord whose "Mod" modifier is Ctrl on Windows/Linux and Command on macOS, plus any extra
/// `alt`/`shift`. This keeps the binding table OS-agnostic (implementation note 1 / RISK-004).
pub fn mod_chord(key: egui::Key, alt: bool, shift: bool) -> KeyChord {
    if mod_is_ctrl() {
        KeyChord { key, ctrl: true, alt, shift, mac_cmd: false }
    } else {
        KeyChord { key, ctrl: false, alt, shift, mac_cmd: true }
    }
}

/// The complete VS-Code-parity command set the code editor dispatches. Each variant is a single,
/// stable command id (the design principle the React `editor_commands.ts` catalog used: the command id
/// is the single source of truth, and the keyboard / palette / AccessKit-command-node / MCP-tool
/// surfaces all reference it). New surfaces add a binding or a node for an existing variant rather than
/// inventing parallel command vocabularies.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CodeEditorAction {
    // ── Caret movement ────────────────────────────────────────────────────────────────────────────
    MoveCursorLeft,
    MoveCursorRight,
    MoveCursorUp,
    MoveCursorDown,
    MoveCursorWordLeft,
    MoveCursorWordRight,
    MoveCursorLineStart,
    MoveCursorLineEnd,
    MoveCursorDocStart,
    MoveCursorDocEnd,
    // ── Selection ─────────────────────────────────────────────────────────────────────────────────
    SelectLeft,
    SelectRight,
    SelectUp,
    SelectDown,
    SelectWordLeft,
    SelectWordRight,
    SelectLineStart,
    SelectLineEnd,
    SelectAll,
    // ── Deletion ──────────────────────────────────────────────────────────────────────────────────
    DeleteLeft,
    DeleteRight,
    DeleteWordLeft,
    DeleteWordRight,
    DeleteLine,
    // ── Insertion / line edits ──────────────────────────────────────────────────────────────────────
    InsertNewline,
    InsertTab,
    IndentLine,
    DedentLine,
    ToggleComment,
    DuplicateLine,
    MoveLineUp,
    MoveLineDown,
    // ── Multi-cursor ──────────────────────────────────────────────────────────────────────────────
    AddCursorAbove,
    AddCursorBelow,
    SelectNextOccurrence,
    CancelMultiCursor,
    // ── Find / replace ────────────────────────────────────────────────────────────────────────────
    OpenFind,
    OpenReplace,
    FindNext,
    FindPrev,
    CloseFind,
    // ── Folding ───────────────────────────────────────────────────────────────────────────────────
    FoldAtCursor,
    UnfoldAtCursor,
    FoldAll,
    UnfoldAll,
    // ── Navigation ────────────────────────────────────────────────────────────────────────────────
    GoToLine,
    GoToDefinition,
    ShowReferences,
    ShowHover,
    // WP-KERNEL-012 MT-048 (E1 — VS Code parity): Rename Symbol (F2). Dispatched by this keymap; the
    // panel calls `rename::begin_rename` on dispatch. The single command id the F2 binding + the editor
    // body context-menu 'Rename Symbol' entry + the AccessKit command node all reference.
    RenameSymbol,
    // WP-KERNEL-012 MT-049 (E1 — VS Code parity): Quick Fix (Ctrl+.). Requests code actions for the
    // current cursor range and opens the quick-fix menu. The single command id the Ctrl+. binding + the
    // editor body context-menu 'Quick Fix...' entry + the AccessKit command node all reference.
    QuickFix,
    // WP-KERNEL-012 MT-050 (E1 — VS Code parity): Format Document (Alt+Shift+F). Requests
    // `textDocument/formatting` and applies the returned TextEdits as one undo step. The single command id
    // the Alt+Shift+F binding + the EDIT-menu / editor body context-menu 'Format Document' entry + the
    // AccessKit command node all reference.
    FormatDocument,
    // WP-KERNEL-012 MT-050 (E1 — VS Code parity): Format Selection (no default binding — menu/context-menu
    // invoked). Requests `textDocument/rangeFormatting` for the current selection (empty selection -> the
    // current line, matching VS Code). The single command id the editor body context-menu 'Format
    // Selection' entry + the AccessKit command node reference.
    FormatSelection,
    // ── Code intelligence ─────────────────────────────────────────────────────────────────────────
    TriggerCompletion,
    AcceptCompletion,
    DismissCompletion,
    // ── History / save / palette ──────────────────────────────────────────────────────────────────
    Undo,
    Redo,
    Save,
    OpenCommandPalette,
    // WP-KERNEL-012 MT-052 (E1 — VS Code parity): GO-menu navigation. APPENDED to the MT-010 enum (do
    // NOT reorder the variants above — RISK-008 / MC-008). F8 = Go to Next Problem, Shift+F8 = Go to
    // Previous Problem (traverse the MT-007 diagnostic markers with wraparound); Alt+Left = Navigate
    // Back, Alt+Right = Navigate Forward (walk the cross-file jump-history stack). The dispatch arms live
    // in panel.rs::process_keymap (where MT-047..051 dispatch), not here — this enum only adds the ids.
    GoToNextDiagnostic,
    GoToPrevDiagnostic,
    NavigateBack,
    NavigateForward,
    // WP-KERNEL-012 MT-053 (E1 — VS Code parity): in-file Go to Symbol (Ctrl+Shift+O). APPENDED to the
    // enum (do NOT reorder the variants above — the same no-reorder discipline MT-052 used). Opens the
    // file-scoped symbol palette (author_id code_editor_symbol_palette), STRICTLY DISTINCT from MT-030's
    // global Ctrl+P/Ctrl+T quick-switcher. The dispatch arm lives in panel.rs::dispatch_action.
    GoToSymbolInFile,
}

impl CodeEditorAction {
    /// Every action variant, in a stable order. Used to emit one AccessKit command node per action
    /// (AC-005) and to enumerate the command surface for the User Manual / MCP tool reference. Keep in
    /// sync with the enum; the unit test `all_covers_every_variant` guards against drift.
    pub fn all() -> &'static [CodeEditorAction] {
        use CodeEditorAction::*;
        &[
            MoveCursorLeft,
            MoveCursorRight,
            MoveCursorUp,
            MoveCursorDown,
            MoveCursorWordLeft,
            MoveCursorWordRight,
            MoveCursorLineStart,
            MoveCursorLineEnd,
            MoveCursorDocStart,
            MoveCursorDocEnd,
            SelectLeft,
            SelectRight,
            SelectUp,
            SelectDown,
            SelectWordLeft,
            SelectWordRight,
            SelectLineStart,
            SelectLineEnd,
            SelectAll,
            DeleteLeft,
            DeleteRight,
            DeleteWordLeft,
            DeleteWordRight,
            DeleteLine,
            InsertNewline,
            InsertTab,
            IndentLine,
            DedentLine,
            ToggleComment,
            DuplicateLine,
            MoveLineUp,
            MoveLineDown,
            AddCursorAbove,
            AddCursorBelow,
            SelectNextOccurrence,
            CancelMultiCursor,
            OpenFind,
            OpenReplace,
            FindNext,
            FindPrev,
            CloseFind,
            FoldAtCursor,
            UnfoldAtCursor,
            FoldAll,
            UnfoldAll,
            GoToLine,
            GoToDefinition,
            ShowReferences,
            ShowHover,
            RenameSymbol,
            QuickFix,
            FormatDocument,
            FormatSelection,
            TriggerCompletion,
            AcceptCompletion,
            DismissCompletion,
            Undo,
            Redo,
            Save,
            OpenCommandPalette,
            // MT-052 (appended — keep after the MT-010/047-050 variants).
            GoToNextDiagnostic,
            GoToPrevDiagnostic,
            NavigateBack,
            NavigateForward,
            // MT-053 (appended — keep after the MT-052 variants).
            GoToSymbolInFile,
        ]
    }

    /// The stable snake_case id for this action. This is the suffix of the AccessKit command node
    /// `code_editor_cmd_{name}` (AC-005) AND the string a [`crate::code_editor::keymap_settings`]
    /// override names in its `action` field (so an operator writes `"OpenFind"`? no — the snake_case id
    /// here, e.g. `"open_find"`). Round-trips with [`CodeEditorAction::from_name`].
    pub fn name(&self) -> &'static str {
        use CodeEditorAction::*;
        match self {
            MoveCursorLeft => "move_cursor_left",
            MoveCursorRight => "move_cursor_right",
            MoveCursorUp => "move_cursor_up",
            MoveCursorDown => "move_cursor_down",
            MoveCursorWordLeft => "move_cursor_word_left",
            MoveCursorWordRight => "move_cursor_word_right",
            MoveCursorLineStart => "move_cursor_line_start",
            MoveCursorLineEnd => "move_cursor_line_end",
            MoveCursorDocStart => "move_cursor_doc_start",
            MoveCursorDocEnd => "move_cursor_doc_end",
            SelectLeft => "select_left",
            SelectRight => "select_right",
            SelectUp => "select_up",
            SelectDown => "select_down",
            SelectWordLeft => "select_word_left",
            SelectWordRight => "select_word_right",
            SelectLineStart => "select_line_start",
            SelectLineEnd => "select_line_end",
            SelectAll => "select_all",
            DeleteLeft => "delete_left",
            DeleteRight => "delete_right",
            DeleteWordLeft => "delete_word_left",
            DeleteWordRight => "delete_word_right",
            DeleteLine => "delete_line",
            InsertNewline => "insert_newline",
            InsertTab => "insert_tab",
            IndentLine => "indent_line",
            DedentLine => "dedent_line",
            ToggleComment => "toggle_comment",
            DuplicateLine => "duplicate_line",
            MoveLineUp => "move_line_up",
            MoveLineDown => "move_line_down",
            AddCursorAbove => "add_cursor_above",
            AddCursorBelow => "add_cursor_below",
            SelectNextOccurrence => "select_next_occurrence",
            CancelMultiCursor => "cancel_multi_cursor",
            OpenFind => "open_find",
            OpenReplace => "open_replace",
            FindNext => "find_next",
            FindPrev => "find_prev",
            CloseFind => "close_find",
            FoldAtCursor => "fold_at_cursor",
            UnfoldAtCursor => "unfold_at_cursor",
            FoldAll => "fold_all",
            UnfoldAll => "unfold_all",
            GoToLine => "go_to_line",
            GoToDefinition => "go_to_definition",
            ShowReferences => "show_references",
            ShowHover => "show_hover",
            RenameSymbol => "rename_symbol",
            QuickFix => "quick_fix",
            FormatDocument => "format_document",
            FormatSelection => "format_selection",
            TriggerCompletion => "trigger_completion",
            AcceptCompletion => "accept_completion",
            DismissCompletion => "dismiss_completion",
            Undo => "undo",
            Redo => "redo",
            Save => "save",
            OpenCommandPalette => "open_command_palette",
            // MT-052 GO-menu navigation.
            GoToNextDiagnostic => "go_to_next_diagnostic",
            GoToPrevDiagnostic => "go_to_prev_diagnostic",
            NavigateBack => "navigate_back",
            NavigateForward => "navigate_forward",
            // MT-053 in-file Go to Symbol.
            GoToSymbolInFile => "go_to_symbol_in_file",
        }
    }

    /// A short human description for the User Manual / palette hint / AccessKit node label.
    pub fn description(&self) -> &'static str {
        use CodeEditorAction::*;
        match self {
            MoveCursorLeft => "Move cursor left",
            MoveCursorRight => "Move cursor right",
            MoveCursorUp => "Move cursor up",
            MoveCursorDown => "Move cursor down",
            MoveCursorWordLeft => "Move cursor word left",
            MoveCursorWordRight => "Move cursor word right",
            MoveCursorLineStart => "Move cursor to line start",
            MoveCursorLineEnd => "Move cursor to line end",
            MoveCursorDocStart => "Move cursor to document start",
            MoveCursorDocEnd => "Move cursor to document end",
            SelectLeft => "Extend selection left",
            SelectRight => "Extend selection right",
            SelectUp => "Extend selection up",
            SelectDown => "Extend selection down",
            SelectWordLeft => "Extend selection word left",
            SelectWordRight => "Extend selection word right",
            SelectLineStart => "Extend selection to line start",
            SelectLineEnd => "Extend selection to line end",
            SelectAll => "Select all",
            DeleteLeft => "Delete character left (Backspace)",
            DeleteRight => "Delete character right (Delete)",
            DeleteWordLeft => "Delete word left",
            DeleteWordRight => "Delete word right",
            DeleteLine => "Delete line",
            InsertNewline => "Insert newline",
            InsertTab => "Insert tab",
            IndentLine => "Indent line",
            DedentLine => "Dedent line",
            ToggleComment => "Toggle line comment",
            DuplicateLine => "Duplicate line",
            MoveLineUp => "Move line up",
            MoveLineDown => "Move line down",
            AddCursorAbove => "Add cursor above",
            AddCursorBelow => "Add cursor below",
            SelectNextOccurrence => "Select next occurrence",
            CancelMultiCursor => "Cancel multi-cursor",
            OpenFind => "Open find",
            OpenReplace => "Open find and replace",
            FindNext => "Find next match",
            FindPrev => "Find previous match",
            CloseFind => "Close find",
            FoldAtCursor => "Fold region at cursor",
            UnfoldAtCursor => "Unfold region at cursor",
            FoldAll => "Fold all regions",
            UnfoldAll => "Unfold all regions",
            GoToLine => "Go to line",
            GoToDefinition => "Go to definition",
            ShowReferences => "Show references",
            ShowHover => "Show hover",
            RenameSymbol => "Rename symbol",
            QuickFix => "Quick fix / code actions",
            FormatDocument => "Format document",
            FormatSelection => "Format selection",
            TriggerCompletion => "Trigger completion",
            AcceptCompletion => "Accept completion",
            DismissCompletion => "Dismiss completion",
            Undo => "Undo",
            Redo => "Redo",
            Save => "Save",
            OpenCommandPalette => "Open command palette",
            // MT-052 GO-menu navigation.
            GoToNextDiagnostic => "Go to next problem",
            GoToPrevDiagnostic => "Go to previous problem",
            NavigateBack => "Navigate back",
            NavigateForward => "Navigate forward",
            // MT-053 in-file Go to Symbol.
            GoToSymbolInFile => "Go to symbol in file",
        }
    }

    /// Parse a snake_case action id (the [`name`](Self::name) form) back to a [`CodeEditorAction`].
    /// Returns `None` for an unknown id (so a settings override naming a removed/typo'd action is
    /// skipped with a warning rather than silently mis-binding — RISK-003 discipline).
    pub fn from_name(name: &str) -> Option<CodeEditorAction> {
        // Linear over the small (<60) action set; building a static map would add a lazy-init dep for
        // no measured win.
        Self::all().iter().copied().find(|a| a.name() == name)
    }
}

/// A single keyboard binding: the [`KeyChord`] that triggers [`CodeEditorAction`]. `prefix` is set when
/// this binding is the FIRST chord of a two-chord sequence (e.g. `Ctrl+K` of `Ctrl+K Ctrl+0`); in that
/// case `chord` is the prefix and `second` is the required follow-up chord. A single-chord binding has
/// `second == None`. This is the Rust port of the React `KeyBinding` interface, extended with the
/// two-chord support VS Code's `Ctrl+K` prefixed commands need.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyBinding {
    /// The triggering chord (or, for a two-chord binding, the PREFIX chord).
    pub chord: KeyChord,
    /// The required SECOND chord for a two-chord binding (`None` for a single-chord binding).
    pub second: Option<KeyChord>,
    /// The command this binding triggers.
    pub action: CodeEditorAction,
    /// A human description (for the palette/manual hint).
    pub description: &'static str,
}

impl KeyBinding {
    /// A single-chord binding.
    pub fn single(chord: KeyChord, action: CodeEditorAction, description: &'static str) -> Self {
        Self { chord, second: None, action, description }
    }

    /// A two-chord binding (`prefix` then `second`).
    pub fn two_chord(
        prefix: KeyChord,
        second: KeyChord,
        action: CodeEditorAction,
        description: &'static str,
    ) -> Self {
        Self { chord: prefix, second: Some(second), action, description }
    }

    /// True when this binding requires a second chord.
    pub fn is_two_chord(&self) -> bool {
        self.second.is_some()
    }
}

/// The editor keymap: an ordered list of [`KeyBinding`]s plus a resolved single-chord lookup map built
/// once for O(1) `resolve`. The binding list is the documentation/UI surface (sorted for legibility);
/// the map is the hot-path resolver.
#[derive(Clone, Debug)]
pub struct Keymap {
    /// The full binding table (single- and two-chord). Kept for the palette/manual hint surface and for
    /// rebuilding the resolver after an override merge.
    bindings: Vec<KeyBinding>,
    /// Single-chord resolver: chord -> action. Built from `bindings` (two-chord bindings are NOT in here
    /// — they resolve through [`resolve_prefix`] + [`resolve_second`]).
    single: HashMap<KeyChord, CodeEditorAction>,
}

impl Keymap {
    /// Build a keymap from an explicit binding list (rebuilds the resolver map). Later bindings WIN over
    /// earlier ones for the same single chord (so an override appended after the defaults takes effect —
    /// this is what [`from_settings`](Self::from_settings) relies on).
    pub fn from_bindings(bindings: Vec<KeyBinding>) -> Self {
        let mut single = HashMap::new();
        for b in &bindings {
            if b.second.is_none() {
                single.insert(b.chord, b.action);
            }
        }
        Self { bindings, single }
    }

    /// The canonical Monaco / VS Code parity binding table (implementation note 1). The "Mod" chords use
    /// [`mod_chord`] so the table is correct on every platform. Sorted loosely by feature group for
    /// documentation legibility.
    pub fn default_vscode() -> Self {
        use egui::Key;
        use CodeEditorAction as A;
        // Helpers to keep the table dense + readable.
        let m = |k: Key| mod_chord(k, false, false);
        let ms = |k: Key| mod_chord(k, false, true); // Mod+Shift
        let ma = |k: Key| mod_chord(k, true, false); // Mod+Alt
        let plain = KeyChord::plain;
        let shift = |k: Key| KeyChord { key: k, ctrl: false, alt: false, shift: true, mac_cmd: false };
        let alt = |k: Key| KeyChord { key: k, ctrl: false, alt: true, shift: false, mac_cmd: false };

        let bindings = vec![
            // ── Find / replace / go-to-line (MT-004 / MT-006) ──
            KeyBinding::single(m(Key::F), A::OpenFind, "Find"),
            KeyBinding::single(m(Key::H), A::OpenReplace, "Find and replace"),
            KeyBinding::single(m(Key::G), A::GoToLine, "Go to line"),
            // ── Multi-cursor (MT-003) ──
            KeyBinding::single(m(Key::D), A::SelectNextOccurrence, "Select next occurrence"),
            KeyBinding::single(ma(Key::ArrowUp), A::AddCursorAbove, "Add cursor above"),
            KeyBinding::single(ma(Key::ArrowDown), A::AddCursorBelow, "Add cursor below"),
            // Escape is context-sensitive (CancelMultiCursor / CloseFind / DismissCompletion) — the
            // dispatcher decides which by current editor state (step 3); the binding maps Escape to the
            // multi-cursor cancel as the default, and the dispatcher overrides per state.
            KeyBinding::single(plain(Key::Escape), A::CancelMultiCursor, "Cancel / close"),
            // ── Folding (MT-005) ──
            KeyBinding::single(ms(Key::OpenBracket), A::FoldAtCursor, "Fold region"),
            KeyBinding::single(ms(Key::CloseBracket), A::UnfoldAtCursor, "Unfold region"),
            // Two-chord folds (VS Code: Ctrl+K Ctrl+0 / Ctrl+K Ctrl+J).
            KeyBinding::two_chord(m(Key::K), m(Key::Num0), A::FoldAll, "Fold all"),
            KeyBinding::two_chord(m(Key::K), m(Key::J), A::UnfoldAll, "Unfold all"),
            // ── Command palette (WP-011 reuse) ──
            KeyBinding::single(ms(Key::P), A::OpenCommandPalette, "Command palette"),
            // ── LSP navigation (MT-008) ──
            KeyBinding::single(plain(Key::F12), A::GoToDefinition, "Go to definition"),
            KeyBinding::single(shift(Key::F12), A::ShowReferences, "Show references"),
            // ── GO-menu navigation (MT-052) ── F8 / Shift+F8 traverse diagnostics; Alt+Left/Right walk
            // the jump-history stack (VS Code parity). Appended after the MT-008 entries so the existing
            // single-chord resolver entries are untouched (RISK-008 — the keymap suite re-proves no
            // regression). Alt+ArrowLeft/Right do not collide with the plain ArrowLeft/Right caret moves
            // (different modifier flags) or with Alt+ArrowUp/Down (different key).
            KeyBinding::single(plain(Key::F8), A::GoToNextDiagnostic, "Go to next problem"),
            KeyBinding::single(shift(Key::F8), A::GoToPrevDiagnostic, "Go to previous problem"),
            KeyBinding::single(alt(Key::ArrowLeft), A::NavigateBack, "Navigate back"),
            KeyBinding::single(alt(Key::ArrowRight), A::NavigateForward, "Navigate forward"),
            // ── In-file Go to Symbol (MT-053) ── Ctrl+Shift+O (Cmd+Shift+O on macOS) opens the
            // file-scoped symbol palette. DISTINCT from MT-030's global quick-switcher (Ctrl+P/Ctrl+T) —
            // different palette, different data scope (RISK-001 / MC-001 / AC-003). Appended after the
            // MT-052 entries so the existing single-chord resolver entries are untouched; ms(Key::O) does
            // not collide with any existing chord (no prior Mod+Shift+O binding).
            KeyBinding::single(ms(Key::O), A::GoToSymbolInFile, "Go to symbol in file"),
            // ── Refactoring (MT-048) ── F2 = Rename Symbol (VS Code parity).
            KeyBinding::single(plain(Key::F2), A::RenameSymbol, "Rename symbol"),
            // ── Quick Fix (MT-049) ── Ctrl+. = code actions / quick-fix menu (VS Code parity).
            KeyBinding::single(m(Key::Period), A::QuickFix, "Quick fix"),
            // ── Formatting (MT-050) ── Alt+Shift+F = Format Document (VS Code parity). Format Selection has
            // NO default binding (menu / context-menu invoked, matching VS Code).
            KeyBinding::single(
                KeyChord { key: Key::F, ctrl: false, alt: true, shift: true, mac_cmd: false },
                A::FormatDocument,
                "Format document",
            ),
            // ── Code intelligence (MT-008) ──
            KeyBinding::single(m(Key::Space), A::TriggerCompletion, "Trigger completion"),
            // Tab is context-sensitive: AcceptCompletion when the popup is open, else InsertTab. The
            // binding maps Tab to InsertTab; the dispatcher promotes it to AcceptCompletion when the
            // popup is open (step 3 precedence).
            KeyBinding::single(plain(Key::Tab), A::InsertTab, "Insert tab / accept completion"),
            // ── History / save ──
            KeyBinding::single(m(Key::Z), A::Undo, "Undo"),
            KeyBinding::single(m(Key::Y), A::Redo, "Redo"),
            KeyBinding::single(ms(Key::Z), A::Redo, "Redo (alt)"),
            KeyBinding::single(m(Key::S), A::Save, "Save"),
            // ── Line edits ──
            KeyBinding::single(m(Key::Slash), A::ToggleComment, "Toggle comment"),
            KeyBinding::single(alt(Key::ArrowUp), A::MoveLineUp, "Move line up"),
            KeyBinding::single(alt(Key::ArrowDown), A::MoveLineDown, "Move line down"),
            KeyBinding::single(ms(Key::K), A::DeleteLine, "Delete line"),
            // ── Selection ──
            KeyBinding::single(m(Key::A), A::SelectAll, "Select all"),
            // ── Caret movement (plain arrows + Home/End) ──
            KeyBinding::single(plain(Key::ArrowLeft), A::MoveCursorLeft, "Move left"),
            KeyBinding::single(plain(Key::ArrowRight), A::MoveCursorRight, "Move right"),
            KeyBinding::single(plain(Key::ArrowUp), A::MoveCursorUp, "Move up"),
            KeyBinding::single(plain(Key::ArrowDown), A::MoveCursorDown, "Move down"),
            KeyBinding::single(m(Key::ArrowLeft), A::MoveCursorWordLeft, "Move word left"),
            KeyBinding::single(m(Key::ArrowRight), A::MoveCursorWordRight, "Move word right"),
            KeyBinding::single(plain(Key::Home), A::MoveCursorLineStart, "Line start"),
            KeyBinding::single(plain(Key::End), A::MoveCursorLineEnd, "Line end"),
            KeyBinding::single(m(Key::Home), A::MoveCursorDocStart, "Document start"),
            KeyBinding::single(m(Key::End), A::MoveCursorDocEnd, "Document end"),
            // ── Selection extension (Shift + movement) ──
            KeyBinding::single(shift(Key::ArrowLeft), A::SelectLeft, "Select left"),
            KeyBinding::single(shift(Key::ArrowRight), A::SelectRight, "Select right"),
            KeyBinding::single(shift(Key::ArrowUp), A::SelectUp, "Select up"),
            KeyBinding::single(shift(Key::ArrowDown), A::SelectDown, "Select down"),
            KeyBinding::single(
                KeyChord { key: Key::Home, ctrl: false, alt: false, shift: true, mac_cmd: false },
                A::SelectLineStart,
                "Select to line start",
            ),
            KeyBinding::single(
                KeyChord { key: Key::End, ctrl: false, alt: false, shift: true, mac_cmd: false },
                A::SelectLineEnd,
                "Select to line end",
            ),
            // ── Editing keys ──
            KeyBinding::single(plain(Key::Backspace), A::DeleteLeft, "Delete left"),
            KeyBinding::single(plain(Key::Delete), A::DeleteRight, "Delete right"),
            KeyBinding::single(plain(Key::Enter), A::InsertNewline, "Insert newline"),
        ];
        Self::from_bindings(bindings)
    }

    /// Resolve a single chord to its bound action (the React `resolveShortcut`). `None` when the chord is
    /// not bound to any single-chord action. Two-chord bindings are resolved via
    /// [`resolve_prefix`](Self::resolve_prefix) + [`resolve_second`](Self::resolve_second); this lookup
    /// deliberately ignores them so a lone `Ctrl+K` does not resolve to a single action.
    pub fn resolve(&self, chord: KeyChord) -> Option<CodeEditorAction> {
        self.single.get(&chord).copied()
    }

    /// True when `chord` is the PREFIX of some two-chord binding (e.g. `Ctrl+K`). The dispatcher uses
    /// this to enter the "pending second chord" state (RISK-001 / MC-001).
    pub fn resolve_prefix(&self, chord: KeyChord) -> bool {
        self.bindings.iter().any(|b| b.second.is_some() && b.chord == chord)
    }

    /// Resolve a `(prefix, second)` two-chord pair to its action. `None` when no two-chord binding has
    /// that exact prefix+second (so a wrong second chord after a prefix clears the pending state with no
    /// action — RISK-001 / MC-001).
    pub fn resolve_second(&self, prefix: KeyChord, second: KeyChord) -> Option<CodeEditorAction> {
        self.bindings
            .iter()
            .find(|b| b.chord == prefix && b.second == Some(second))
            .map(|b| b.action)
    }

    /// The full binding table (for the palette/manual hint surface + the AccessKit/MCP command
    /// reference). Read-only.
    pub fn bindings(&self) -> &[KeyBinding] {
        &self.bindings
    }

    /// All bindings for a given action (the React `bindingsForAction`) — for showing the chord hint in
    /// the UI / manual.
    pub fn bindings_for_action(&self, action: CodeEditorAction) -> Vec<KeyBinding> {
        self.bindings.iter().copied().filter(|b| b.action == action).collect()
    }

    /// Merge operator overrides over the default table (the MT `Keymap::from_settings`). Each override's
    /// chord is parsed and, if valid, APPENDED as a single-chord binding for the named action — appended
    /// AFTER the defaults so it WINS the single-chord resolver (later bindings overwrite earlier ones in
    /// [`from_bindings`](Self::from_bindings)). An override whose chord or action name does not parse is
    /// SKIPPED with a warning (RISK-003 / MC-002 — never a silent wrong binding). The default table is
    /// the base, so an unspecified action keeps its VS Code default.
    pub fn from_settings(settings: &super::keymap_settings::KeymapSettings) -> Self {
        let mut bindings = Self::default_vscode().bindings;
        for ov in &settings.overrides {
            match (
                super::keymap_settings::KeymapSettings::chord_from_str(&ov.chord),
                CodeEditorAction::from_name(&ov.action),
            ) {
                (Ok(chord), Some(action)) => {
                    bindings.push(KeyBinding::single(chord, action, "Operator override"));
                }
                (Err(e), _) => {
                    tracing::warn!(
                        chord = %ov.chord,
                        action = %ov.action,
                        error = %e,
                        "skipping keymap override: unparseable chord"
                    );
                }
                (_, None) => {
                    tracing::warn!(
                        chord = %ov.chord,
                        action = %ov.action,
                        "skipping keymap override: unknown action id"
                    );
                }
            }
        }
        Self::from_bindings(bindings)
    }
}

impl Default for Keymap {
    fn default() -> Self {
        Self::default_vscode()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Key;

    #[test]
    fn all_covers_every_variant_and_names_are_unique() {
        let all = CodeEditorAction::all();
        // 65 variants in the contract enum (56 base + MT-048 RenameSymbol + MT-049 QuickFix + MT-050
        // FormatDocument + FormatSelection + MT-052 GoToNextDiagnostic/GoToPrevDiagnostic/NavigateBack/
        // NavigateForward + MT-053 GoToSymbolInFile).
        assert_eq!(all.len(), 65, "all() must list every variant exactly once");
        let mut names: Vec<&str> = all.iter().map(|a| a.name()).collect();
        names.sort_unstable();
        let before = names.len();
        names.dedup();
        assert_eq!(before, names.len(), "every action name() is unique");
    }

    #[test]
    fn name_round_trips_through_from_name() {
        for action in CodeEditorAction::all() {
            assert_eq!(
                CodeEditorAction::from_name(action.name()),
                Some(*action),
                "{} round-trips",
                action.name()
            );
        }
        assert_eq!(CodeEditorAction::from_name("no_such_action"), None);
    }

    #[test]
    fn default_table_resolves_core_chords() {
        let km = Keymap::default_vscode();
        // Mod = Ctrl on the CI/dev host (Windows/Linux).
        let ctrl = |k: Key| KeyChord { key: k, ctrl: true, alt: false, shift: false, mac_cmd: false };
        assert_eq!(km.resolve(ctrl(Key::F)), Some(CodeEditorAction::OpenFind));
        assert_eq!(km.resolve(ctrl(Key::H)), Some(CodeEditorAction::OpenReplace));
        assert_eq!(km.resolve(ctrl(Key::G)), Some(CodeEditorAction::GoToLine));
        assert_eq!(km.resolve(ctrl(Key::D)), Some(CodeEditorAction::SelectNextOccurrence));
        assert_eq!(km.resolve(KeyChord::plain(Key::F12)), Some(CodeEditorAction::GoToDefinition));
        assert_eq!(km.resolve(ctrl(Key::S)), Some(CodeEditorAction::Save));
    }

    #[test]
    fn unbound_chord_resolves_to_none() {
        let km = Keymap::default_vscode();
        let weird = KeyChord { key: Key::Q, ctrl: true, alt: true, shift: true, mac_cmd: false };
        assert_eq!(km.resolve(weird), None);
    }

    #[test]
    fn two_chord_prefix_does_not_resolve_as_single() {
        let km = Keymap::default_vscode();
        let ctrl_k = KeyChord { key: Key::K, ctrl: true, alt: false, shift: false, mac_cmd: false };
        // Ctrl+K is a prefix, not a single action.
        assert_eq!(km.resolve(ctrl_k), None);
        assert!(km.resolve_prefix(ctrl_k), "Ctrl+K is a two-chord prefix");
        let ctrl_0 = KeyChord { key: Key::Num0, ctrl: true, alt: false, shift: false, mac_cmd: false };
        let ctrl_j = KeyChord { key: Key::J, ctrl: true, alt: false, shift: false, mac_cmd: false };
        assert_eq!(km.resolve_second(ctrl_k, ctrl_0), Some(CodeEditorAction::FoldAll));
        assert_eq!(km.resolve_second(ctrl_k, ctrl_j), Some(CodeEditorAction::UnfoldAll));
        // A wrong second chord after the prefix resolves to nothing (pending cleared, no action).
        let ctrl_x = KeyChord { key: Key::X, ctrl: true, alt: false, shift: false, mac_cmd: false };
        assert_eq!(km.resolve_second(ctrl_k, ctrl_x), None);
    }
}
