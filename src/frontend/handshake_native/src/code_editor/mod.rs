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
pub mod cursor;
pub mod find_replace;
pub mod folding;
pub mod gutter;
pub mod highlight;
pub mod minimap;
pub mod outline;
pub mod panel;
pub mod virtual_lines;

pub use breakpoints::{BreakpointAction, BreakpointEvent, BreakpointSet};
pub use buffer::{BufferError, TextBuffer};
pub use gutter::{
    DiagnosticSeverity, Gutter, GutterConfig, GutterGeometry, GutterMarker, GutterMarkerKind,
    GutterResponse, BREAKPOINT_COLOR,
};
pub use find_replace::{FindEngine, FindQuery, Match, MAX_PATTERN_LEN};
pub use cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet, MoveDir,
    MAX_ACCESSKIT_CURSORS,
};
pub use folding::{FoldProvider, FoldRegion, FoldSet, FoldableNodeTypes};
pub use highlight::{
    language_id_for_extension, HighlightScope, HighlightSpan, Highlighter, LanguageRegistry,
    SafeLanguage,
};
pub use minimap::{Minimap, MinimapResponse, DEFAULT_MINIMAP_WIDTH};
pub use outline::{OutlineItem, OutlineKind, OutlineProvider};
pub use panel::{
    scope_to_color, CodeEditorPanel, CodeEditorPaneFactory, FindState, GotoLineState, PerfStats,
    CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX, CODE_EDITOR_CURSOR_AUTHOR_PREFIX,
    CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX, CODE_EDITOR_FIND_BAR_AUTHOR_ID,
    CODE_EDITOR_FIND_NEXT_AUTHOR_ID, CODE_EDITOR_FIND_PREV_AUTHOR_ID, CODE_EDITOR_GOTO_LINE_AUTHOR_ID,
    CODE_EDITOR_GUTTER_AUTHOR_ID, CODE_EDITOR_MINIMAP_AUTHOR_ID, CODE_EDITOR_OUTLINE_AUTHOR_ID,
    CODE_EDITOR_PANEL_AUTHOR_ID, CODE_EDITOR_REPLACE_BAR_AUTHOR_ID, CODE_EDITOR_SCROLL_AREA_AUTHOR_ID,
    CODE_EDITOR_TEXT_AUTHOR_ID,
};
pub use virtual_lines::{VirtualLineLayout, OVERSCAN_LINES};
