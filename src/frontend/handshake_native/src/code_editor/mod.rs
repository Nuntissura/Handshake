//! Native code-editor surface (WP-KERNEL-012 E1 — VS Code parity).
//!
//! MT-001 lays the foundation every later E1 microtask builds on:
//! - [`buffer::TextBuffer`] — the rope-backed, byte-addressed document buffer (sole text owner).
//! - [`highlight`] — the tree-sitter highlight pipeline ([`Highlighter`], [`HighlightScope`],
//!   [`HighlightSpan`], [`LanguageRegistry`]).
//! - [`panel::CodeEditorPanel`] — the egui widget that renders highlighted lines and exposes the
//!   AccessKit nodes a swarm agent addresses.
//!
//! MT-002 adds [`virtual_lines::VirtualLineLayout`] — the viewport-line virtualization calculator —
//! and rewires the panel to paint only the visible line window via `egui::ScrollArea::show_rows`, so
//! a 100k-line file stays under the per-frame budget.
//!
//! MT-003 adds [`cursor::CursorSet`] — the multi-cursor + column/box selection model (the Helix
//! `Selection` shape: a `Vec` of `(anchor, head)` byte-offset [`cursor::Cursor`]s) — and wires
//! Alt+Click / Ctrl+Alt+Up/Down / Alt+Shift drag / Ctrl+D into the panel, painting every caret +
//! selection over the virtualized rows and emitting a capped set of `Role::Caret` AccessKit nodes
//! (`code_editor_cursor_{n}`; the contract named `Role::TextCursor`, which does not exist in accesskit
//! 0.21 — `Caret` is the field-correct caret role) a swarm agent addresses.
//!
//! MT-005 adds [`folding`] — tree-sitter-derived code folding ([`FoldProvider`], [`FoldRegion`],
//! [`FoldSet`], [`FoldableNodeTypes`]). Foldable regions are identified from the SAME parse tree the
//! highlighter builds (exposed via [`highlight::Highlighter::tree`]); a folded region collapses to a
//! single summary line, the panel skips the hidden lines in the virtualized layout, and each foldable
//! region surfaces a `Role::TreeItem` AccessKit node (`code_editor_fold_{start_line}`) with
//! Expand/Collapse actions so a swarm agent can fold/unfold by id.
//!
//! MT-006 adds the three navigation aids that port the React editor workbench chrome into the native
//! editor: [`minimap`] (a scaled-down whole-file overview with a viewport indicator + click-to-scroll —
//! `Role::ScrollBar` node `code_editor_minimap`), [`outline`] (the tree-sitter symbol tree with
//! click-to-scroll — `Role::Tree` node `code_editor_outline`), and a go-to-line palette (Ctrl+G —
//! `Role::TextInput` node `code_editor_goto_line`). The outline reuses the SAME tree-sitter tree + the
//! same `TreeCursor` pre-order walk pattern as [`folding`] (MC-002), and all three navigation actions
//! map through the MT-002-corrected positioning units + the MT-005 fold-aware visible<->buffer line
//! mapping so navigation lands on the correct line when folds are active.
//!
//! MT-007 adds the [`gutter`] — the left-margin strip rendering line numbers, diagnostic severity
//! icons, fold triangles (MT-005 fold state), and breakpoint toggles ([`breakpoints::BreakpointSet`]).
//! It aligns row-for-row with the editor body via the SAME painted-row geometry (`RowGeometry`) +
//! MT-005 fold-aware visible<->buffer mapping, recomputes its width every frame from the live line
//! count (RISK-001), and surfaces a `Role::Group` strip node (`code_editor_gutter`), a
//! `Role::CheckBox` toggle per breakpoint line (`code_editor_breakpoint_{line}`), and a `Role::Label`
//! per diagnostic line (`code_editor_diagnostic_{line}`) — the field-correct accesskit 0.21.1 roles for
//! the contract's `Role::ToggleButton`/`StaticText`, the same documented-deviation pattern MT-003
//! (`TextCursor`->`Caret`) used. A toggled breakpoint publishes a [`breakpoints::BreakpointEvent`] onto
//! an unbounded `std::sync::mpsc` channel for the future debug-adapter MT (RISK-003: non-blocking
//! `send().ok()`). `CodeEditorPanel::push_diagnostics` is the slot MT-008's LSP client fills.
//!
//! Later E1 MTs add the LSP client (MT-008), the diff editor (MT-009), and the Monaco-parity keymap
//! (MT-010) on top of these primitives. They REUSE the WP-011 shell modules (`pane_registry`,
//! `split_layout`, `theme/*`, `accessibility/*`, `backend_client`), which these MTs also reuse rather
//! than re-creating.

pub mod breakpoints;
pub mod buffer;
// WP-KERNEL-012 MT-049 (E1 — VS Code parity): Code actions / quick fixes. The lightbulb that turns a
// read-only diagnostic into an actionable fix. Owns the code-action request lifecycle, the action-list +
// menu state, the gutter-lightbulb decision, and the apply call — which DELEGATES to the MT-048
// WorkspaceEdit apply path (`rename::apply_text_edits_to_buffer`), never re-implementing it. Builds on the
// MT-007 gutter diagnostic store (the cursor-rest trigger), the MT-008 LSP transport (`code_action`), and
// the MT-048 multi-file/atomic apply (so cross-file fixes apply, not silently dropped).
pub mod code_actions;
pub mod code_nav;
pub mod cursor;
pub mod diff_editor_panel;
pub mod diff_engine;
pub mod editor_view;
pub mod find_replace;
pub mod folding;
// WP-KERNEL-012 MT-050 (E1 — VS Code parity): Format Document (Alt+Shift+F) + Format Selection. Owns the
// LSP-TextEdit-to-buffer applier (descending-offset, UTF-16-correct), the single-undo grouping at the
// panel boundary, the formatter-capability gate, and the menu descriptors. DELEGATES the descending-offset
// apply discipline to the same pattern MT-048's `rename::apply_text_edits_to_buffer` established; binds the
// MT-008 LSP transport (`format_document`/`format_range`) additively — no second transport.
pub mod formatting;
pub mod gutter;
pub mod highlight;
pub mod keymap;
pub mod keymap_settings;
pub mod lsp_client;
pub mod minimap;
// WP-KERNEL-012 MT-034 (E5 — code<->note cross-refs): the "Notes mentioning this symbol" side panel.
// The native-only reverse-direction surface (the React CodeSymbolPanel has only the definition + file
// lens). Lists rich docs that reference the focused symbol via interop::find_notes_referencing_symbol;
// clicking a row dispatches the shared open-document command on the MT-031 bus.
pub mod note_refs_panel;
pub mod outline;
pub mod panel;
// WP-KERNEL-012 MT-048 (E1 — VS Code parity): Rename Symbol (F2). The rename state machine, the inline
// rename input widget, the multi-file WorkspaceEdit preview model, and the descending-offset/atomic-write
// apply logic. Builds on the MT-001 tree-sitter highlight tree (identifier resolution) + ropey TextBuffer
// (apply) + the MT-008 LSP transport (textDocument/rename). A pure addition; mutates the operator's source
// files, so descending-offset apply (RISK-001) + atomic temp+rename disk writes (RISK-002) are the bar.
pub mod rename;
// WP-KERNEL-012 MT-047 (E1 — VS Code parity): LSP signature help (parameter hints) + the code-nav
// fallback signature, rendered as a floating popup above the cursor with the active parameter
// emphasized. A pure addition on the MT-008 LSP transport + code-nav client.
pub mod signature_help;
pub mod virtual_lines;

pub use breakpoints::{BreakpointAction, BreakpointEvent, BreakpointSet};
pub use buffer::{BufferError, TextBuffer};
pub use code_actions::{
    draw_lightbulb, lightbulb_color, normalize_code_actions, quickfix_item_author_id,
    quickfix_lightbulb_author_id, render_menu, AppliedAction, CodeActionController, CodeActionError,
    CodeActionItem, CodeActionResult, CodeActionState, LspCommand, MenuAction,
    CODE_EDITOR_QUICKFIX_ITEM_AUTHOR_PREFIX, CODE_EDITOR_QUICKFIX_LIGHTBULB_AUTHOR_PREFIX,
    CODE_EDITOR_QUICKFIX_MENU_AUTHOR_ID, MAX_QUICKFIX_NODES, NO_ACTIONS_TEXT,
};
pub use diff_editor_panel::{
    build_line_map, right_line_for_left_line, DiffEditorPaneFactory, DiffEditorPanel, DiffMode, Side,
    SyncRow, SyncScrollState, DIFF_BLOCK_ACCEPT_LOCAL_PREFIX, DIFF_EDITOR_PANEL_AUTHOR_ID,
    DIFF_MODE_TOGGLE_AUTHOR_ID,
};
pub use diff_engine::{
    diff_json_blocks, DiffBlock, DiffEngine, DiffStatus, MergeBlock, MergeChoice, MergeEngine,
    MergeStatus,
};
pub use code_nav::{
    code_symbol_staleness_label, markdown_for_symbol, staleness_marker_for, symbol_file_path,
    CodeFileLensResponse, CodeNavCache, CodeNavClient, CodeStaleness, CodeSymbolDefinition,
    CodeSymbolNavProjection, CodeSymbolReferencesResponse, CodeSymbolResponse, CompletionItem,
    CompletionKind, HoverResult as CodeNavHoverResult, Location as CodeNavLocation,
    COMPLETION_DEBOUNCE_MS, HOVER_DWELL_MS, LOOKUP_CACHE_TTL, SYMBOL_LOOKUP_LIMIT,
};
pub use editor_view::{
    CompletionOutcome, CompletionPopup, CompletionState, HoverOutcome, HoverState, HoverTooltip,
    CODE_EDITOR_COMPLETION_ITEM_AUTHOR_PREFIX, CODE_EDITOR_COMPLETION_POPUP_AUTHOR_ID,
    CODE_EDITOR_HOVER_AUTHOR_ID, CODE_EDITOR_HOVER_GOTODEF_AUTHOR_ID,
};
pub use lsp_client::{
    published_diagnostics_from_lsp, HoverResult as LspHoverResult, LspClient, LspCompletionItem,
    LspDiagnostic, LspError, LspServerConfig, PublishedDiagnostics, REQUEST_TIMEOUT, SHUTDOWN_TIMEOUT,
};
pub use gutter::{
    breakpoint_color, diagnostic_tokens_for, DiagnosticSeverity, Gutter, GutterConfig,
    GutterGeometry, GutterMarker, GutterMarkerKind, GutterResponse,
};
pub use find_replace::{FindEngine, FindQuery, Match, MAX_PATTERN_LEN};
pub use formatting::{
    apply_text_edits, apply_text_edits_to_string as apply_format_edits_to_string,
    byte_to_lsp_position, default_formatting_options, formatter_available, lsp_range_to_byte_range,
    menu_descriptors as format_menu_descriptors, range_formatter_available, resolve_format_outcome,
    selection_range_for, FormatApplyError, FormatMenuDescriptor, FormatOutcome,
    FORMAT_DOCUMENT_CTX_AUTHOR_ID, FORMAT_DOCUMENT_MENU_AUTHOR_ID, FORMAT_SELECTION_CTX_AUTHOR_ID,
    NO_FORMATTER_TOOLTIP,
};
pub use cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet, MoveDir,
    MAX_ACCESSKIT_CURSORS,
};
pub use folding::{FoldProvider, FoldRegion, FoldSet, FoldableNodeTypes};
pub use highlight::{
    language_id_for_extension, HighlightScope, HighlightSpan, Highlighter, LanguageRegistry,
    SafeLanguage,
};
pub use keymap::{
    mod_chord, mod_is_ctrl, CodeEditorAction, KeyBinding, KeyChord, Keymap,
};
pub use keymap_settings::{
    key_from_str, keymap_settings_path, KeymapOverride, KeymapSettings, KeymapSettingsError,
};
pub use minimap::{Minimap, MinimapResponse, DEFAULT_MINIMAP_WIDTH};
pub use note_refs_panel::{
    render_note_refs_panel, row_author_id as note_ref_row_author_id,
    NoteRefsState, PANEL_AUTHOR_ID as NOTE_REFS_PANEL_AUTHOR_ID,
};
pub use outline::{OutlineItem, OutlineKind, OutlineProvider};
pub use rename::{
    apply_preview, apply_text_edits_to_buffer, apply_text_edits_to_string, begin_rename,
    identifier_occurrences, identifier_range_at, is_identifier_kind, references_lack_precise_ranges,
    render_inline_input, render_no_lsp_banner, render_preview, write_file_atomic, FileEditPreview,
    LspRange, PreviewAction, PreviewHunk, RenameApplyReport, RenameError, RenameState,
    TextEdit as RenameTextEdit, WorkspaceEditPreview, CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
    CODE_EDITOR_RENAME_APPLY_AUTHOR_ID, CODE_EDITOR_RENAME_CANCEL_AUTHOR_ID,
    CODE_EDITOR_RENAME_INPUT_AUTHOR_ID, CODE_EDITOR_RENAME_NO_LSP_BANNER_AUTHOR_ID, NO_LSP_BANNER_TEXT,
};
pub use signature_help::{
    active_parameter_from_commas, render_signature_popup, signature_from_code_nav_symbol,
    signature_label_runs, ParamSpan, SignatureHelpState, SignatureInfo, SignatureSource,
    CODE_EDITOR_SIGNATURE_HELP_AUTHOR_ID, SIGNATURE_HELP_TRIGGER_CHARS,
};
pub use panel::{
    scope_to_color, CodeEditorPanel, CodeEditorPaneFactory, FindState, GotoLineState, PerfStats,
    CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX, CODE_EDITOR_COMMAND_AUTHOR_PREFIX,
    CODE_EDITOR_CURSOR_AUTHOR_PREFIX, CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX,
    CODE_EDITOR_FIND_BAR_AUTHOR_ID, CODE_EDITOR_FIND_NEXT_AUTHOR_ID, CODE_EDITOR_FIND_PREV_AUTHOR_ID,
    CODE_EDITOR_GOTO_LINE_AUTHOR_ID, CODE_EDITOR_GUTTER_AUTHOR_ID, CODE_EDITOR_MINIMAP_AUTHOR_ID,
    CODE_EDITOR_OUTLINE_AUTHOR_ID, CODE_EDITOR_PANEL_AUTHOR_ID, CODE_EDITOR_REPLACE_BAR_AUTHOR_ID,
    CODE_EDITOR_SCROLL_AREA_AUTHOR_ID, CODE_EDITOR_TEXT_AUTHOR_ID, TWO_CHORD_TIMEOUT,
};
pub use virtual_lines::{VirtualLineLayout, OVERSCAN_LINES};

use std::sync::Arc;

/// Construct a diff editor surface for two documents and wrap it in a [`DiffEditorPaneFactory`] so it
/// mounts in the WP-011 docking split layout through the EXISTING `pane_registry` + `split_layout`
/// (no new shell layout system — MT-009 contract). Returns the factory ready to register against a
/// pane record; the caller (the shell) adds it to its factory set the same way it registers
/// [`CodeEditorPaneFactory`]. `extension` selects the grammar for both panes.
///
/// MT contract: `open_diff(left: TextBuffer, right: TextBuffer)`.
pub fn open_diff(left: TextBuffer, right: TextBuffer, extension: &str) -> DiffEditorPaneFactory {
    DiffEditorPaneFactory::new(DiffEditorPanel::diff(left, right, extension))
}

/// Construct a three-way merge editor surface (local/base/remote) wrapped in a
/// [`DiffEditorPaneFactory`] for the same EXISTING pane registry + split layout. The merge view opens
/// with the local/remote panes plus per-conflict accept controls (MT step 5).
///
/// MT contract: `open_merge(base, local, remote: TextBuffer)`.
pub fn open_merge(
    base: TextBuffer,
    local: TextBuffer,
    remote: TextBuffer,
    extension: &str,
) -> DiffEditorPaneFactory {
    DiffEditorPaneFactory::new(DiffEditorPanel::merge(base, local, remote, extension))
}

/// A shared handle to a [`DiffEditorPanel`] for callers (and tests) that need to drive the panel
/// (accept a merge choice, toggle mode, scroll) after [`open_diff`] / [`open_merge`] mounts it. The
/// factory keeps an `Arc` to the same panel it renders, so this clone observes the live state.
pub fn diff_panel_handle(factory: &DiffEditorPaneFactory) -> Arc<DiffEditorPanel> {
    factory.panel()
}

/// MT-031 (E5 melt-together): the code editor's thin adapter into the shared
/// [`crate::interop::InteractionBus`]. The code editor routes its Copy/Cut/Paste/SelectAll/Find
/// through the ONE shared command + clipboard surface instead of owning ad-hoc per-pane clipboard
/// state — the contract's "no per-pane ad-hoc clipboard" rule (AC-7). The functions here are the
/// concrete `bus.register_command` + `bus.clipboard_write` call sites for the code surface.
pub mod interop_adapter {
    use crate::code_editor::panel::CodeEditorPanel;
    use crate::interop::adapters::{
        copy_selection_to_clipboard, register_standard_commands, text_range_selection,
    };
    use crate::interop::interaction_bus::{EditorSurfaceKind, InteractionBus, SharedSelection};
    use crate::pane_registry::PaneId;
    use crate::rich_editor::properties::metadata_client::ClipboardSink;

    /// Register the code surface's melt-together command set into the shared bus (AC-4). Called once
    /// when the code pane mounts. The Copy/Cut/Paste edit commands stay addressable + keybind-matchable;
    /// the pane wires their buffer-specific behavior through the keybind path + [`copy_to_bus`].
    pub fn register(bus: &mut InteractionBus) {
        register_standard_commands(bus, EditorSurfaceKind::Code);
    }

    /// Materialize the panel's PRIMARY selection as a [`SharedSelection::TextRange`] (the code pane
    /// publishes this to the bus when its selection changes). Returns [`SharedSelection::None`] for a
    /// bare caret (no selected text). The selected text is sliced from the buffer BY BYTE RANGE
    /// (O(selection-length) via [`CodeEditorPanel::selected_primary_text`]) — it NEVER `.to_string()`s the
    /// whole document, so publishing on each selection change in a multi-MB file stays cheap (the
    /// perf-lens cap / RISK-003). The range is clamped defensively (a stale range never panics — RISK-4).
    pub fn selection_for(panel: &CodeEditorPanel, pane_id: PaneId) -> SharedSelection {
        match panel.selected_primary_text() {
            Some((start, end, text)) => {
                text_range_selection(pane_id, EditorSurfaceKind::Code, start, end, text)
            }
            None => SharedSelection::None,
        }
    }

    /// Publish the code pane's current selection to the shared bus + run the keybind dispatch — the LIVE
    /// per-frame wiring [`CodeEditorPaneFactory::render`] calls so a MOUNTED code pane is a real bus
    /// consumer (not test-only dead code). On the first call it registers the code surface's command set
    /// (idempotent — last-registration-wins by id). `has_focus` gates focus ownership so a background pane
    /// never clobbers the focused pane's selection (impl note 6/7). All bus access is via `with_try_lock`
    /// so it never blocks the egui frame thread (RISK-1 / MC-1).
    pub fn drive_bus_in_render(
        bus: &std::sync::Arc<std::sync::Mutex<InteractionBus>>,
        panel: &CodeEditorPanel,
        pane_id: PaneId,
        has_focus: bool,
        already_registered: &mut bool,
    ) {
        let registered = *already_registered;
        InteractionBus::with_try_lock(bus, |b| {
            if !registered {
                register(b);
            }
            if has_focus {
                b.set_focus_owner(pane_id.clone());
                let selection = selection_for(panel, pane_id.clone());
                b.set_selection(selection);
            }
        });
        *already_registered = true;
    }

    /// Copy the code pane's current selection to the shared clipboard through the bus (Ctrl+C path).
    /// Returns `true` when text was copied. The OS write goes through the mockable [`ClipboardSink`]
    /// (headless-safe — MT-017 precedent), and the bus caches the variant for a cross-pane Paste.
    pub fn copy_to_bus(
        bus: &mut InteractionBus,
        panel: &CodeEditorPanel,
        pane_id: PaneId,
        sink: &dyn ClipboardSink,
    ) -> bool {
        let selection = selection_for(panel, pane_id);
        copy_selection_to_clipboard(bus, &selection, sink)
    }

    /// Paste the shared clipboard's text into the code pane at its cursors (Ctrl+V path). Reads the
    /// richest cross-pane variant from the bus (so a `loom://` ref pastes its URI text). Returns the
    /// number of insertions applied.
    pub fn paste_from_bus(bus: &InteractionBus, panel: &CodeEditorPanel) -> usize {
        match bus.clipboard_read_text() {
            Some(text) if !text.is_empty() => panel.insert_text(&text),
            _ => 0,
        }
    }

    use crate::undo_stack::{UndoAction, UndoResult};
    use std::sync::{Arc, Weak};

    /// MT-035 (E5 unified undo): record a LOCAL code-edit undo action on the shared scope for `pane_id`
    /// (POLICY-1). `before` is the rope snapshot taken BEFORE the edit (ropey clones are O(1) — impl note
    /// 1/2, safe to clone per edit), `after` is the snapshot AFTER. The undo_fn restores `before`; the
    /// redo_fn re-applies `after`. Both capture a `Weak<CodeEditorPanel>` back-ref (RISK-3 / MC-3): they
    /// upgrade only during invocation and report [`UndoResult::pane_dropped`] (a benign no-op) if the
    /// code pane was closed, so a stale undo never panics and never forms a retain cycle with the
    /// panel the host holds. The MT-001 `TextBuffer` deliberately has NO per-editor undo stack (it routes
    /// Ctrl+Z to the bus), so THIS is the integration point that makes Ctrl+Z real for the code pane —
    /// no second parallel undo stack is added (the wrap-not-fork discipline).
    pub fn push_code_edit_undo(
        bus: &mut InteractionBus,
        pane_id: PaneId,
        panel: &Arc<CodeEditorPanel>,
        before: crate::code_editor::TextBuffer,
        after: crate::code_editor::TextBuffer,
        description: impl Into<String>,
    ) {
        let weak: Weak<CodeEditorPanel> = Arc::downgrade(panel);
        let undo_weak = weak.clone();
        let before_text = before.to_string();
        let after_text = after.to_string();
        let undo_fn: crate::undo_stack::UndoFn = Arc::new(move || match undo_weak.upgrade() {
            Some(p) => {
                p.set_text(&before_text);
                UndoResult::ok()
            }
            None => UndoResult::pane_dropped(),
        });
        let redo_fn: crate::undo_stack::UndoFn = Arc::new(move || match weak.upgrade() {
            Some(p) => {
                p.set_text(&after_text);
                UndoResult::ok()
            }
            None => UndoResult::pane_dropped(),
        });
        bus.push_undo_local(pane_id, UndoAction::sync(description, undo_fn, redo_fn));
    }
}
