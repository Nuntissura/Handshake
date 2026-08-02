//! egui widget that renders the native code editor panel (WP-KERNEL-012 MT-001 + MT-002).
//!
//! [`CodeEditorPanel`] owns a [`TextBuffer`] + a [`Highlighter`] and paints the visible lines with
//! per-scope theme colors. It exposes three stable AccessKit nodes a swarm agent addresses:
//! - an OUTER `Role::GenericContainer` node with `author_id = "code_editor_panel"` (the panel frame),
//! - a `Role::ScrollView` node with `author_id = "code_editor_scroll_area"` (the virtualized scroll
//!   region — MT-002), and
//! - an INNER `Role::TextInput` node with `author_id = "editor.code.text"` (the editable text area),
//!   each emitted INSIDE its parent's egui scope so the live AccessKit tree links them
//!   container -> scroll-area -> text (the same nesting linkage the WP-011 shell relies on).
//!
//! ## Theme-driven colors (no hardcoded hex)
//!
//! [`scope_to_color`] maps each [`HighlightScope`] to a color taken from the active theme's
//! [`HsSyntaxTokens`] (`theme/syntax.rs`). The panel reads the live `egui::Visuals` to decide
//! dark/light and pulls the matching token set, so it never embeds a `Color32` literal (the
//! no-hardcode invariant the theme layer enforces).
//!
//! ## Viewport virtualization (MT-002 — replaces the MT-001 render cap)
//!
//! [`CodeEditorPanel::show`] paints the document through `egui::ScrollArea::vertical().show_rows(..)`,
//! the idiomatic native virtualization primitive (RESEARCH-PROVENANCE wf_ffa74d6d 2026-06-22:
//! confirmed for egui 0.33; no custom painter needed for read/highlight virtualization). `show_rows`
//! sizes the content rect to the WHOLE document (so the scrollbar thumb is proportioned correctly)
//! but only invokes the row closure for the lines that intersect the viewport, so a 100k-line file
//! renders a few dozen lines per frame instead of all of them. The MT-001 `MAX_RENDERED_LINES` cap is
//! gone — virtualization makes it unnecessary.
//!
//! ## Diagnostics surface reflects egui's ACTUAL painted range (AC-007)
//!
//! [`perf_stats`](CodeEditorPanel::perf_stats) and
//! [`last_visible_range`](CodeEditorPanel::last_visible_range) report the EXACT row range
//! `show_rows` painted this frame — the `row_range` egui passes to the paint closure — NOT a separate
//! recompute. egui derives that range INSIDE `show_rows` from the live viewport using
//! `row_height_with_spacing = line_height + item_spacing.y` and applies NO overscan (egui 0.33.3
//! `scroll_area.rs:948-963`). Capturing egui's own range (rather than re-deriving it with
//! [`VirtualLineLayout`](super::virtual_lines::VirtualLineLayout), which adds ±`OVERSCAN_LINES` and
//! uses the sans-spacing height) is what lets the swarm-diagnostics count and the overlay-positioning
//! seam MT-003+ builds on match the pixels on screen line-for-line.
//!
//! [`VirtualLineLayout`](super::virtual_lines::VirtualLineLayout) is retained ONLY as the headless,
//! GPU-free calculator for the AC-001 boundary math and for `total_height_px`/`y_for_line`; it is no
//! longer driven on the live render path and does not feed the diagnostics.
//!
//! ## Highlight cache (MT-002 — recompute only when the buffer changes)
//!
//! Highlighting is cached behind a `buffer_version` counter: [`refresh`](CodeEditorPanel::refresh)
//! bumps the version and recomputes, and the render path reuses the cached spans while the version is
//! unchanged — so spans are NOT recomputed every frame. (Edits land in MT-003; the version counter is
//! the invalidation hook those edits will bump — RISK-002, including on undo/redo.)
//!
//! ## author_id instance suffix (RISK-004)
//!
//! Multiple panels (e.g. a diff view mounting two editors) would collide on the fixed author_ids.
//! Each [`CodeEditorPanel`] carries an `instance` string; [`CodeEditorPanel::with_instance`] appends
//! it (`code_editor_panel#<instance>`) so concurrently-mounted panels stay individually addressable.
//! The default (single) panel uses the bare ids the MT contract names so AC-005 matches exactly. Each
//! instance also gets a unique `egui::Id` so two panels never fight over one `ScrollArea` scroll
//! state (RISK-004).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use egui::accesskit;
use tree_sitter::Tree;

use crate::accessibility::editor_action_registry::{
    CodeDispatch, EditorActionRegistry, EditorActionState, PaneType as EditorPaneType,
    RegistrationHandle, CODE_ACTION_CATALOG,
};
use crate::interop::cross_ref::{
    find_code_ref_notes_with, FindNotesHttp, FindNotesSearch, SymbolDwellTracker,
};
use crate::interop::InteractionBus;
use crate::mcp::action::{serialize_observer_click_state, ClickCompletionState};
use crate::pane_registry::{PaneFactory, PaneId, PaneRenderContext, PaneType};
use crate::theme::HsSyntaxTokens;

use super::note_refs_panel::{render_note_refs_panel, NoteRefsState, OPEN_PENDING_AUTHOR_ID};

use super::buffer::TextBuffer;
use super::code_nav::{
    preferred_symbol_for_identifier_in_file, staleness_marker_for, symbol_file_path, CodeNavCache,
    CodeNavClient, CodeSymbolNavProjection, CodeSymbolReferencesResponse, CompletionItem,
    COMPLETION_DEBOUNCE_MS, HOVER_DWELL_MS, SYMBOL_LOOKUP_LIMIT,
};
use super::cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet,
    MAX_ACCESSKIT_CURSORS,
};
use super::editor_view::{
    completion_item_author_id, CodeNavigationLocation, CompletionOutcome, CompletionPopup,
    CompletionState, HoverOutcome, HoverState, HoverTooltip, CODE_EDITOR_COMPLETION_ACCEPT_EFFECT,
    CODE_EDITOR_COMPLETION_OBSERVER_AUTHOR_ID,
};
use super::formatting::{self, FormatOutcome};

/// A host-routed editor command with the stable document identity that emitted it. Detached windows
/// and docked panes share one channel, so the action alone is insufficient to select the correct
/// buffer when the host drains commands after rendering multiple panes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeEditorHostCommand {
    pub action: CodeEditorAction,
    pub document_id: String,
    pub pane_id: Option<PaneId>,
}

static CODE_PANEL_INCARNATION_COUNTER: AtomicU64 = AtomicU64::new(1);
// MT-051 line-edit buffer transforms: the dispatch arms for ToggleComment / DuplicateLine / MoveLine /
// DeleteLine / Indent / Dedent / InsertTab call into this module (pure TextBuffer + CursorSet transforms).
use super::breakpoints::{BreakpointAction, BreakpointEvent, BreakpointSet};
use super::code_actions::{self, AppliedAction, CodeActionController, MenuAction};
use super::cursor::MoveDir;
use super::find_replace::{FindEngine, FindQuery, Match, REPLACE_ALL_CAP};
use super::folding::{FoldProvider, FoldSet};
use super::gutter::{
    DiagnosticSeverity, Gutter, GutterConfig, GutterGeometry, GutterMarker, GutterMarkerKind,
    GutterPaintRow, GutterResponse,
};
use super::highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry};
use super::jump_history::{JumpEntry, JumpHistory};
use super::keymap::{CodeEditorAction, KeyChord, Keymap};
use super::keymap_settings::{keymap_settings_path, KeymapSettings};
use super::line_ops;
use super::lsp_client::{
    LspClient, LspCompletionItem, PublishedDiagnostics, MAX_LSP_CONTENT_BYTES,
};
use super::minimap::Minimap;
use super::navigation::{next_diagnostic, prev_diagnostic, BufferPosition};
use super::outline::{OutlineItem, OutlineProvider};
use super::rename::{self, PreviewAction, RenameApplyReport, RenameState, WorkspaceEditPreview};
use super::signature_help::{
    active_parameter_from_commas, render_signature_popup, SignatureHelpState,
};
// MT-054 editor-chrome decorations: bracket match / pair-colorize + indent-guide geometry, and the
// word-wrap VisualRow layout math. Pure over the buffer; the panel paint path consumes them.
use super::render_decorations::{
    bracket_pair_colors_in_segments, find_matching_bracket, indent_guide_x, indent_level_of,
    BracketMatch,
};
use super::word_wrap::{count_visual_rows_for_line, layout_visual_rows, VisualRow, WrapConfig};

/// The MT-contract author_id for the outer panel container (AC-005: Role::GenericContainer).
pub const CODE_EDITOR_PANEL_AUTHOR_ID: &str = "code_editor_panel";
/// The MT-002 author_id for the virtualized scroll region (AC-004: Role::ScrollView).
pub const CODE_EDITOR_SCROLL_AREA_AUTHOR_ID: &str = "code_editor_scroll_area";
/// The MT-contract author_id for the inner editable text area (AC-005: Role::TextInput).
pub const CODE_EDITOR_TEXT_AUTHOR_ID: &str = "editor.code.text";
/// The MT-003 author_id PREFIX for each multi-cursor node (AC-004: `code_editor_cursor_{n}`). Cursor
/// `n` (sorted index) gets `code_editor_cursor_{n}` with accesskit `Role::Caret` (the field-correct
/// caret role in accesskit 0.21 — the contract's `Role::TextCursor` does not exist there). Only the
/// first [`MAX_ACCESSKIT_CURSORS`] cursors are surfaced (RISK-004 / MC-004).
pub const CODE_EDITOR_CURSOR_AUTHOR_PREFIX: &str = "code_editor_cursor_";

const CODE_TEXT_UNDO_BATCH_WINDOW: Duration = Duration::from_millis(500);

/// MT-004 find/replace author_ids. The find input is `code_editor_find_bar` (the MT contract names
/// `Role::SearchBox`, which does NOT exist in accesskit 0.21 — `Role::SearchInput` is the field-correct
/// equivalent; see `emit_find_bar_nodes` for the documented deviation). The replace input is
/// `code_editor_replace_bar` (`Role::TextInput`) and the Next button is `code_editor_find_next`
/// (`Role::Button`). The Prev button reuses the same pattern with a fresh slot.
pub const CODE_EDITOR_FIND_BAR_AUTHOR_ID: &str = "code_editor_find_bar";
pub const CODE_EDITOR_REPLACE_BAR_AUTHOR_ID: &str = "code_editor_replace_bar";
pub const CODE_EDITOR_FIND_NEXT_AUTHOR_ID: &str = "code_editor_find_next";
pub const CODE_EDITOR_FIND_PREV_AUTHOR_ID: &str = "code_editor_find_prev";

/// MT-108 (MT-004 residual, RISK-004): find-bar geometry used both to render the pinned bar and to
/// inset the "scroll to current match" so a match never lands hidden BEHIND the bar. The bar is pinned
/// to the top of the editor viewport; scrolling a match to the very top (`line * line_height`) would
/// occlude it. [`CodeEditorPanel::scroll_to_match_line`] subtracts the bar height (taller in replace
/// mode) plus the top margin and a small gap so the current match lands just below the bar. These MUST
/// stay in sync with `render_find_bar`'s `bar_height` / `bar_min` values.
const FIND_BAR_HEIGHT_SINGLE_PX: f32 = 34.0;
const FIND_BAR_HEIGHT_REPLACE_PX: f32 = 64.0;
const FIND_BAR_TOP_MARGIN_PX: f32 = 4.0;
const FIND_BAR_MATCH_REVEAL_GAP_PX: f32 = 6.0;

/// The MT-005 author_id PREFIX for each foldable-region node (AC-005: `code_editor_fold_{start_line}`).
/// Region starting on buffer line `L` gets `code_editor_fold_{L}` with accesskit `Role::TreeItem` and
/// an `Action::Expand` (when folded) or `Action::Collapse` (when unfolded) action a swarm agent
/// dispatches to fold/unfold by id. Only the foldable regions inside the painted window are surfaced
/// (capped — RISK-001) so a 1000-fold file does not emit 1000 nodes per frame.
pub const CODE_EDITOR_FOLD_AUTHOR_PREFIX: &str = "code_editor_fold_";
pub const CODE_EDITOR_FOLD_TARGET_AUTHOR_PREFIX: &str = "code_editor_fold_target_";

/// MT-006 navigation-aid author_ids (AC-003/004/005). The minimap node is `code_editor_minimap`
/// (`Role::ScrollBar` — clicking scrolls; the role exists in accesskit 0.21.1, no fallback needed); the
/// outline tree is `code_editor_outline` (`Role::Tree`); the go-to-line input is `code_editor_goto_line`
/// (`Role::TextInput`). All three roles named in the MT contract exist in accesskit 0.21.1 (verified
/// against the locked source), so unlike the MT-003 TextCursor / MT-004 SearchBox cases no role fallback
/// is required for this MT.
pub const CODE_EDITOR_MINIMAP_AUTHOR_ID: &str = "code_editor_minimap";
pub const CODE_EDITOR_OUTLINE_AUTHOR_ID: &str = "code_editor_outline";
pub const CODE_EDITOR_GOTO_LINE_AUTHOR_ID: &str = "code_editor_goto_line";
pub const CODE_EDITOR_TOGGLE_OUTLINE_AUTHOR_ID: &str = "code_editor_toggle_outline";
pub const CODE_EDITOR_TOGGLE_MINIMAP_AUTHOR_ID: &str = "code_editor_toggle_minimap";
pub const CODE_EDITOR_TOGGLE_NOTE_REFS_AUTHOR_ID: &str = "code_editor_toggle_note_refs";
pub const CODE_EDITOR_VISIBLE_WRAP_TOGGLE_AUTHOR_ID: &str = "code_editor_toggle_wrap";
pub const CODE_EDITOR_KEYBINDINGS_AUTHOR_ID: &str = "code_editor_keybindings";
pub const CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID: &str = "code_editor_context_surface";
pub const CODE_EDITOR_OUTLINE_ROW_AUTHOR_PREFIX: &str = "code_editor_outline_row_";

/// MT-053 in-file Go-to-Symbol palette author_ids (AC-003 / AC-005 / MC-005). The palette list
/// container is `code_editor_symbol_palette` (`Role::List`); the search input is
/// `code_editor_symbol_palette_search` (`Role::TextInput`); each result row is `symbol-{index}`
/// (`Role::ListItem`). These are the exact ids the MT contract names so a swarm agent addresses the
/// palette + its rows. The search/list/dialog are FIXED-band nodes (default panel); the per-row + the
/// sticky-header nodes are DYNAMIC (count varies with the filter / scroll) and live in egui's hashed id
/// space addressed by these stable strings, the same pattern as the fold/command per-item nodes.
pub const CODE_EDITOR_SYMBOL_PALETTE_AUTHOR_ID: &str = "code_editor_symbol_palette";
pub const CODE_EDITOR_SYMBOL_PALETTE_SEARCH_AUTHOR_ID: &str = "code_editor_symbol_palette_search";
pub const CODE_EDITOR_SYMBOL_ROW_AUTHOR_PREFIX: &str = "symbol-";

/// MT-053 sticky-scroll author_ids (AC-004 / AC-006 / MC-005). The pinned band container is
/// `code_editor_sticky_scroll` (`Role::GenericContainer`); each pinned header is `sticky-header-{depth}`
/// (`Role::Button`, so a swarm agent can click a header to scroll to its scope). The header nodes are
/// DYNAMIC (count varies with the scroll position, capped at `max_sticky_lines`) and live in egui's
/// hashed id space addressed by these stable strings.
pub const CODE_EDITOR_STICKY_SCROLL_AUTHOR_ID: &str = "code_editor_sticky_scroll";
pub const CODE_EDITOR_STICKY_HEADER_AUTHOR_PREFIX: &str = "sticky-header-";

/// Max in-file symbol-palette result-row AccessKit nodes emitted per frame (RISK / node-budget cap, the
/// analog of the cursor/fold caps). Only the first this-many filtered rows get a `symbol-{index}` node so
/// a pathological generated file cannot blow the per-frame node budget; the list itself shows them all in
/// a ScrollArea.
pub const MAX_ACCESSKIT_SYMBOL_ROWS: usize = 128;

/// MT-007 gutter author_ids (AC-005 / AC-003). The gutter strip is `code_editor_gutter` (the MT names
/// `Role::Group`, which exists in accesskit 0.21.1 — no fallback). Each breakpoint toggle is
/// `code_editor_breakpoint_{line}` (the MT names `Role::ToggleButton`, which does NOT exist in
/// accesskit 0.21.1 — `Role::CheckBox` is the field-correct toggle-state role, exposing `set_toggled`;
/// AC-005 asserts the author_id + the toggled state change, not the role string, so the CheckBox
/// satisfies it — the same documented-deviation pattern as MT-003's `TextCursor`->`Caret`). Each
/// diagnostic marker is `code_editor_diagnostic_{line}` (the MT names `Role::StaticText`, which does
/// NOT exist in accesskit 0.21.1 — `Role::Label` is the field-correct static-text role).
pub const CODE_EDITOR_GUTTER_AUTHOR_ID: &str = "code_editor_gutter";
/// MT-046 IC-09: stable prefix for an interactive diagnostic related-note chip. The zero-based line is
/// appended, and panel instances receive the normal `#instance` suffix.
pub const CODE_EDITOR_DIAGNOSTIC_NOTE_REF_AUTHOR_PREFIX: &str = "code_editor_diagnostic_note_ref_";
pub const CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX: &str = "code_editor_breakpoint_";
pub const CODE_EDITOR_BREAKPOINT_TARGET_AUTHOR_PREFIX: &str = "code_editor_breakpoint_target_";
pub const CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX: &str = "code_editor_diagnostic_";

/// Max foldable-region AccessKit nodes emitted per frame (RISK-001 / RISK-004 analog of the cursor
/// cap). Only the regions intersecting the painted window are emitted, capped at this many so a
/// pathological file with thousands of folds cannot blow the per-frame node budget.
pub const MAX_ACCESSKIT_FOLDS: usize = 64;

/// MT-049: the cursor-rest debounce window (ms) before a passive `textDocument/codeAction` request fires
/// on a diagnostic line (RISK-001 / MC-001 — only fire once the cursor has settled + only on a line that
/// carries >=1 diagnostic, never per idle frame). ~300ms matches the VS Code lightbulb dwell.
pub const CODE_ACTION_REST_MS: u64 = 300;

/// MT-034: the bounded timeout (ms) for the BEST-EFFORT word->symbol_key resolution the code->notes
/// dwell does before the find-notes search. If the code-nav backend is slow/unreachable, the dwell falls
/// back to searching the raw caret word rather than pinning the NoteRefsPanel in Loading on a stuck
/// connect (the MT-015 no-perpetual-spinner lesson — the off-thread task always completes promptly).
pub const SYMBOL_KEY_LOOKUP_TIMEOUT_MS: u64 = 1500;

/// MT-006 navigation-aid fixed AccessKit `NodeId`s for the default (single-instance) panel. A fresh
/// band (370..372) never collides with the container/scroll/text (200/201/202), cursor (210..274),
/// find-bar (280..283), or dynamic fold nodes. Multi-instance panels hash the suffixed author_id instead
/// (RISK-004), the same scheme every other panel node uses.
const PANEL_MINIMAP_NODE_ID: u64 = 370;
const PANEL_OUTLINE_NODE_ID: u64 = 371;
const PANEL_GOTO_LINE_NODE_ID: u64 = 372;

/// MT-053 fixed AccessKit `NodeId`s for the default (single-instance) panel. A fresh band (700..702)
/// ABOVE the MT-010 command band (600..600+N≈660), disjoint from the container/scroll/text (200/201/202),
/// cursor (210..274), find-bar (280..283), nav (370..372), gutter (400/410../480..), and command (600..)
/// bands; dynamic fold nodes are keyed by fold start line instead of a fixed band. The symbol-palette
/// dialog (the modal window scope), the palette list container, and the search input get fixed ids; the
/// sticky band container gets a fixed id too. Per-row + per-header nodes are DYNAMIC (hashed id space).
/// Multi-instance panels hash the suffixed author_id instead (RISK-004), the same scheme every other
/// panel node uses.
const PANEL_SYMBOL_PALETTE_DIALOG_NODE_ID: u64 = 700;
const PANEL_SYMBOL_PALETTE_LIST_NODE_ID: u64 = 701;
const PANEL_SYMBOL_PALETTE_SEARCH_NODE_ID: u64 = 702;
const PANEL_STICKY_SCROLL_NODE_ID: u64 = 703;

/// Max per-line breakpoint / diagnostic AccessKit nodes emitted per frame (RISK-004 analog of the
/// cursor/fold caps). Only the breakpoints/diagnostics on the painted rows are emitted, capped so a
/// file with thousands of either cannot blow the per-frame node budget.
pub const MAX_ACCESSKIT_GUTTER_MARKERS: usize = 64;

/// MT-007 gutter fixed AccessKit `NodeId`s for the default (single-instance) panel. Fresh bands ABOVE
/// the MT-006 nav band (370..372): the gutter strip Group at 400; the per-line breakpoint `CheckBox`
/// nodes in 410..410+MAX_ACCESSKIT_GUTTER_MARKERS; the per-line diagnostic `Label` nodes in
/// 480..480+MAX_ACCESSKIT_GUTTER_MARKERS — all disjoint from the container/scroll/text (200/201/202),
/// cursor (210..274), find-bar (280..283), and nav (370..372) bands; dynamic fold nodes are keyed by
/// fold start line. Multi-instance panels hash the suffixed author_id instead (RISK-004).
const PANEL_GUTTER_NODE_ID: u64 = 400;
const PANEL_BREAKPOINT_NODE_ID_BASE: u64 = 410;
const PANEL_DIAGNOSTIC_NODE_ID_BASE: u64 = 480;

/// MT-010 author_id PREFIX for each editor-command AccessKit node (AC-005:
/// `code_editor_cmd_{action_name}`). For every [`CodeEditorAction`] variant the panel emits a hidden
/// `Role::Button` node named `code_editor_cmd_{snake_case_action}` (e.g. `code_editor_cmd_open_find`)
/// with NO visual area — invisible to the human operator but addressable by a swarm agent (HBR-SWARM)
/// so an agent can dispatch any editor command by id WITHOUT simulating a keystroke. The same action
/// set is the MCP swarm tool surface. The nodes are CACHED outside the per-frame render hot loop and
/// rebuilt only when the keymap changes (RISK-002 / MC: do not emit 56 fresh nodes every frame).
pub const CODE_EDITOR_COMMAND_AUTHOR_PREFIX: &str = "code_editor_cmd_";

/// MT-010 fixed AccessKit `NodeId` band for the per-command `Role::Button` nodes (default
/// single-instance panel): 600..600+N (N = number of [`CodeEditorAction`] variants). A fresh band ABOVE
/// the gutter diagnostic band (480..544) so the command nodes never collide with the container/scroll/
/// text (200/201/202), cursor (210..274), find-bar (280..283), nav (370..372), or gutter
/// (400/410../480..) bands; dynamic fold nodes are keyed by fold start line. Multi-instance panels hash
/// the suffixed author_id instead (RISK-004).
const PANEL_COMMAND_NODE_ID_BASE: u64 = 600;

/// How often the keymap override file (`~/.handshake/keymap.json`) is polled for changes, in seconds
/// (implementation note 6). A cheap mtime stat — NOT the `notify` crate (the contract says avoid adding
/// `notify` when it is not already in the dependency tree; it is not). When the mtime moves the keymap
/// is reloaded + the cached command nodes are rebuilt on the next frame.
const KEYMAP_RELOAD_POLL_SECS: u64 = 5;

/// The fixed AccessKit `NodeId` band the per-cursor `Role::Caret` nodes occupy for the default panel
/// (210..210+MAX_ACCESSKIT_CURSORS), disjoint from the panel container/scroll/text band (200/201/202)
/// and the WP-011 shell band (>= 100).
const PANEL_CURSOR_NODE_ID_BASE: u64 = 210;

/// Fixed AccessKit `NodeId`s for the MT-004 find-bar controls (default single-instance panel). A fresh
/// band (280..283) ABOVE the cursor band (210..274) so the find-bar nodes never collide with the
/// container/scroll/text nodes or the per-cursor caret nodes. Multi-instance panels hash the suffixed
/// author_id instead (RISK-004), the same scheme the cursor/container ids use.
const PANEL_FIND_BAR_NODE_ID: u64 = 280;
const PANEL_REPLACE_BAR_NODE_ID: u64 = 281;
const PANEL_FIND_NEXT_NODE_ID: u64 = 282;
const PANEL_FIND_PREV_NODE_ID: u64 = 283;

/// Find-match highlight colors. These are UI affordances (like egui's own selection bg), NOT syntax
/// tokens, so — exactly like the MT-003 selection overlay tint — they are the one place this MT
/// specifies explicit RGBA the contract names: a translucent YELLOW over every match and translucent
/// ORANGE over the current match (AC-005). They are intentionally distinct from the cornflower-blue
/// selection tint so a match never reads as a selection.
const MATCH_HIGHLIGHT_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(180, 160, 0, 110);
const CURRENT_MATCH_HIGHLIGHT_COLOR: egui::Color32 =
    egui::Color32::from_rgba_premultiplied(200, 120, 0, 150);

/// The monospace font size the panel renders text at (matches `render_line`). Centralized so the caret
/// overlay measures glyph width with the SAME `FontId` the glyphs are painted with (MT-003 positioning
/// requirement — no x-unit drift). `pub(crate)` so the MT-007 gutter paints its line numbers / fold
/// triangles with the SAME monospace metrics the editor body uses (row-for-row alignment).
pub(crate) const MONO_FONT_SIZE: f32 = 13.0;

/// Fixed AccessKit `NodeId`s for the default (single-instance) panel. They sit in a fresh band
/// (200/201/202) ABOVE the WP-011 pane id space (>= 100) so they cannot collide with shell chrome,
/// dividers, or panes. Multi-instance panels (RISK-004) derive their ids by hashing the suffixed
/// author_id into egui's hashed id space instead of this fixed band.
const PANEL_CONTAINER_NODE_ID: u64 = 200;
const PANEL_TEXT_NODE_ID: u64 = 201;
const PANEL_SCROLL_NODE_ID: u64 = 202;

/// MT-054 word-wrap toggle AccessKit node id (default single-instance panel). A fresh slot (290) ABOVE
/// the find-bar band (280..283) so it never collides with any other panel node; multi-instance panels
/// hash the suffixed author_id instead (RISK-004), the same scheme the other panel nodes use.
pub const EDITOR_WRAP_TOGGLE_NODE_ID: u64 = 290;

/// MT-054 the contract-named stable author_id for the word-wrap toggle node. A swarm agent flips wrap
/// deterministically by addressing THIS id (the MT names it exactly `editor-wrap-toggle`).
pub const CODE_EDITOR_WRAP_TOGGLE_AUTHOR_ID: &str = "editor-wrap-toggle";

/// MT-046 the stable id for the editor-body context menu's 'Copy as note reference' entry (the code ->
/// note interconnection edge: selection/identifier -> `[[code:…]]` ref onto the SHARED InteractionBus
/// clipboard). Follows the `code_editor_ctx_{action}` scheme its sibling entries (rename / quick-fix /
/// format-selection) use.
pub const CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID: &str = "code_editor_ctx_copy_note_ref";

/// Per-frame virtualization diagnostics for the swarm/debug surface (MT-002 step 4). Reports how many
/// lines were actually painted this frame versus the document size, so a no-context model (or a perf
/// test) can confirm virtualization is active (`frame_lines_rendered` << `buffer_len_lines` on a
/// large document) without scraping pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PerfStats {
    /// Number of document lines the row closure painted on the most recent frame — exactly
    /// `row_range.len()` for the range `egui::ScrollArea::show_rows` passed to the closure (AC-007),
    /// or 0 if the panel has not rendered yet. egui applies NO overscan to this range, so this is the
    /// true on-screen line count, not a padded estimate.
    pub frame_lines_rendered: usize,
    /// Total lines in the buffer (the whole document).
    pub buffer_len_lines: usize,
    /// MT-054 perf cap: the number of LOGICAL buffer lines whose bytes were materialized + wrapped by the
    /// per-frame PAINT path this frame. Under word wrap this MUST stay O(painted window), NOT O(document)
    /// — the wrap VisualRow list for the painted window is built lazily from only the logical lines that
    /// intersect the on-screen visual-row range, never the whole post-fold document (the perf regression
    /// the adversarial review caught). `0` when wrap is off (the non-wrap render path materializes lines
    /// the same way `render_rows` always has) or before the first frame. A perf test asserts this is
    /// bounded by the painted window even on a large wrapped document.
    pub frame_lines_wrapped: usize,
}

#[derive(Clone, Debug, Default)]
struct HighlightSpanWindow {
    spans: Vec<HighlightSpan>,
    prefix_max_end: Vec<usize>,
}

impl HighlightSpanWindow {
    fn from_spans(spans: Vec<HighlightSpan>) -> Self {
        let mut prefix_max_end = Vec::with_capacity(spans.len());
        let mut max_end = 0usize;
        for span in &spans {
            max_end = max_end.max(span.byte_range.end);
            prefix_max_end.push(max_end);
        }
        Self {
            spans,
            prefix_max_end,
        }
    }

    fn overlapping(
        &self,
        win_start: usize,
        win_end: usize,
    ) -> impl Iterator<Item = &HighlightSpan> {
        let bounds = if win_end <= win_start {
            0..0
        } else {
            let begin = self
                .prefix_max_end
                .partition_point(|max_end| *max_end <= win_start);
            let end = self
                .spans
                .partition_point(|span| span.byte_range.start < win_end);
            begin..end
        };
        self.spans[bounds]
            .iter()
            .filter(move |span| span.byte_range.end > win_start)
    }
}

const INITIAL_HIGHLIGHT_PENDING: u8 = 0;
const INITIAL_HIGHLIGHT_COMPLETE: u8 = 1;
const INITIAL_HIGHLIGHT_FAILED: u8 = 2;
const INITIAL_HIGHLIGHT_QUEUE_CAPACITY: usize = 8;
const INITIAL_HIGHLIGHT_MAX_ATTEMPTS: u8 = 2;
const INITIAL_HIGHLIGHT_MAX_SOURCE_BYTES: usize = MAX_LSP_CONTENT_BYTES;

fn initial_highlight_source_is_worker_eligible(source_bytes: usize) -> bool {
    source_bytes <= INITIAL_HIGHLIGHT_MAX_SOURCE_BYTES
}

/// Stable, race-proof state for the large-document initial highlight projection. `Complete` is stored
/// with release ordering only after the completed span window has been installed in the cache.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialHighlightStatus {
    Pending,
    Complete,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InitialHighlightFailure {
    WorkerUnavailable,
    WorkerPanicked,
    QueueSaturated,
    HighlighterUnavailable,
    EmptyProjection,
    Cancelled,
    SourceTooLarge,
    StaleDelivery,
}

enum InitialHighlightDelivery {
    Success {
        version: u64,
        generation: u64,
        window: HighlightSpanWindow,
    },
    Error {
        version: u64,
        generation: u64,
        failure: InitialHighlightFailure,
    },
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitialHighlightTestFault {
    SpawnUnavailable,
    QueueFull,
    Disconnect,
    WorkerPanicked,
    EmptyProjection,
    StaleGeneration,
    CancelDuringCapture,
}

struct InitialHighlightJob {
    source: Arc<[u8]>,
    tree: Tree,
    extension: String,
    version: u64,
    generation: u64,
    had_initial_spans: bool,
    cancel: Arc<std::sync::atomic::AtomicBool>,
    result_tx: mpsc::Sender<InitialHighlightDelivery>,
    #[cfg(test)]
    test_fault: Option<InitialHighlightTestFault>,
}

fn initial_highlight_worker_sender() -> Option<&'static mpsc::SyncSender<InitialHighlightJob>> {
    static SENDER: std::sync::OnceLock<Option<mpsc::SyncSender<InitialHighlightJob>>> =
        std::sync::OnceLock::new();
    SENDER
        .get_or_init(|| {
            let (tx, rx) =
                mpsc::sync_channel::<InitialHighlightJob>(INITIAL_HIGHLIGHT_QUEUE_CAPACITY);
            std::thread::Builder::new()
                .name("code-highlight-worker".to_owned())
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        let delivery = if job.cancel.load(Ordering::Acquire) {
                            InitialHighlightDelivery::Error {
                                version: job.version,
                                generation: job.generation,
                                failure: InitialHighlightFailure::Cancelled,
                            }
                        } else {
                            let projected =
                                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    #[cfg(test)]
                                    if job.test_fault
                                        == Some(InitialHighlightTestFault::WorkerPanicked)
                                    {
                                        panic!("injected initial-highlight worker panic");
                                    }
                                    let highlighter = LanguageRegistry::with_bundled_languages()
                                        .highlighter_for_extension(&job.extension)
                                        .ok_or(InitialHighlightFailure::HighlighterUnavailable)?;
                                    let mut cancellation_checks = 0_usize;
                                    let spans = highlighter.captures_for_tree_cancellable(
                                        &job.tree,
                                        &job.source,
                                        0..job.source.len(),
                                        || {
                                            cancellation_checks += 1;
                                            job.cancel.load(Ordering::Acquire)
                                                || cfg!(test)
                                                    && {
                                                        #[cfg(test)]
                                                        {
                                                            job.test_fault
                                                                == Some(InitialHighlightTestFault::CancelDuringCapture)
                                                                && cancellation_checks > 4
                                                        }
                                                        #[cfg(not(test))]
                                                        {
                                                            false
                                                        }
                                                    }
                                        },
                                    )
                                    .ok_or(InitialHighlightFailure::Cancelled)?;
                                    #[cfg(test)]
                                    if job.test_fault
                                        == Some(InitialHighlightTestFault::EmptyProjection)
                                    {
                                        return Err(InitialHighlightFailure::EmptyProjection);
                                    }
                                    if spans.is_empty() && job.had_initial_spans {
                                        return Err(InitialHighlightFailure::EmptyProjection);
                                    }
                                    Ok(HighlightSpanWindow::from_spans(spans))
                                }));
                            match projected {
                                Ok(Ok(window)) if !job.cancel.load(Ordering::Acquire) => {
                                    InitialHighlightDelivery::Success {
                                        version: job.version,
                                        generation: {
                                            #[cfg(test)]
                                            if job.test_fault
                                                == Some(InitialHighlightTestFault::StaleGeneration)
                                            {
                                                job.generation.saturating_add(1)
                                            } else {
                                                job.generation
                                            }
                                            #[cfg(not(test))]
                                            {
                                                job.generation
                                            }
                                        },
                                        window,
                                    }
                                }
                                Ok(Ok(_)) => InitialHighlightDelivery::Error {
                                    version: job.version,
                                    generation: job.generation,
                                    failure: InitialHighlightFailure::Cancelled,
                                },
                                Ok(Err(failure)) => InitialHighlightDelivery::Error {
                                    version: job.version,
                                    generation: job.generation,
                                    failure,
                                },
                                Err(_) => InitialHighlightDelivery::Error {
                                    version: job.version,
                                    generation: job.generation,
                                    failure: InitialHighlightFailure::WorkerPanicked,
                                },
                            }
                        };
                        let _ = job.result_tx.send(delivery);
                    }
                })
                .ok()
                .map(|_| tx)
        })
        .as_ref()
}

/// Map a [`HighlightScope`] to a color from the active theme's syntax tokens — NEVER a hardcoded hex
/// literal. `Other` falls back to the editor foreground (`punctuation` token, which the theme derives
/// from the palette's `text_subtle`). Backed by the theme layer per the MT implementation note.
pub fn scope_to_color(scope: HighlightScope, syntax: &HsSyntaxTokens) -> egui::Color32 {
    match scope {
        HighlightScope::Keyword => syntax.keyword,
        HighlightScope::String => syntax.string,
        HighlightScope::Comment => syntax.comment,
        HighlightScope::Number => syntax.number,
        HighlightScope::Type => syntax.type_name,
        // The grammar has no dedicated function/operator token in the shared theme set yet; reuse the
        // closest existing semantic token (function reads as a type-like accent; operator as
        // punctuation). Keeping these theme-sourced preserves the no-hardcode invariant.
        HighlightScope::Function => syntax.type_name,
        HighlightScope::Operator => syntax.punctuation,
        HighlightScope::Other => syntax.punctuation,
    }
}

/// Resolve the active theme's syntax tokens from the live egui visuals (dark vs light) so the panel's
/// colors track the shell theme without threading the whole palette through every call site.
fn syntax_tokens_for(visuals: &egui::Visuals) -> HsSyntaxTokens {
    if visuals.dark_mode {
        crate::theme::HsTheme::Dark.palette().syntax
    } else {
        crate::theme::HsTheme::Light.palette().syntax
    }
}

/// Map an LSP `publishDiagnostics` payload to MT-007 gutter markers (AC-008). The LSP severity integers
/// (1=Error, 2=Warning, 3=Information, 4=Hint) map onto the gutter's [`DiagnosticSeverity`]; the LSP
/// `range.start.line` is already 0-based (the gutter's coordinate space).
fn lsp_diagnostics_to_markers(published: &PublishedDiagnostics) -> Vec<GutterMarker> {
    published
        .diagnostics
        .iter()
        .map(|d| {
            let severity = match d.severity {
                1 => DiagnosticSeverity::Error,
                2 => DiagnosticSeverity::Warning,
                3 => DiagnosticSeverity::Info,
                4 => DiagnosticSeverity::Hint,
                _ => DiagnosticSeverity::Error,
            };
            GutterMarker::diagnostic(d.line, severity, d.message.clone())
        })
        .collect()
}

/// MT-047: find the byte offset of the open-paren of the call whose argument list `prefix` ends inside
/// (i.e. the cursor sits just after `prefix`), or `None` when `prefix` does not end inside an unclosed
/// `(`. Scans LEFT from the end of `prefix`, balancing `)` against `(` so a nested CLOSED call is
/// skipped; the first `(` with no matching later `)` is the active call's open-paren. String and char
/// literals are respected (a `(` inside a string is not a call). Used to anchor + dismiss the popup.
fn find_enclosing_open_paren(prefix: &str) -> Option<usize> {
    // Walk forward tracking literal state + an explicit stack of open-paren byte offsets; a `)` pops.
    // The TOP of the stack at the end is the enclosing call's open-paren (the cursor is inside it).
    let mut stack: Vec<usize> = Vec::new();
    let mut in_string = false;
    let mut in_char = false;
    let mut escaped = false;
    for (i, c) in prefix.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if in_char {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '\'' {
                in_char = false;
            }
            continue;
        }
        match c {
            '"' => in_string = true,
            '\'' => in_char = true,
            '(' => stack.push(i),
            ')' => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.pop()
}

/// MT-047: the identifier token immediately preceding the end of `prefix` (e.g. the `add` in
/// `... = add` when `prefix` is the text up to a call's `(`), or an empty string when the last
/// non-whitespace run is not an identifier. Trailing whitespace before the identifier is skipped so
/// `add (` still resolves `add`.
fn identifier_before(prefix: &str) -> String {
    let chars: Vec<char> = prefix.chars().collect();
    let mut end = chars.len();
    // Skip trailing whitespace.
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    let mut start = end;
    while start > 0 {
        let c = chars[start - 1];
        if c.is_alphanumeric() || c == '_' {
            start -= 1;
        } else {
            break;
        }
    }
    chars[start..end].iter().collect()
}

/// MT-054 PERF CAP: the cache key the [`WrapRowIndex`] is valid for. The index must be rebuilt whenever
/// any input that changes the per-line wrap-row counts moves — a buffer edit (`buffer_version`), a fold
/// expand/collapse (`fold_version` + the visible-line count), the wrap toggle / column / viewport width
/// (`WrapConfig`), or a font-metric change (`glyph_width`). f32 inputs are keyed by their raw bit pattern
/// so an exact-equality compare is well-defined (NaN never equals NaN, forcing a safe rebuild).
#[derive(Clone, Copy, PartialEq, Eq)]
struct WrapRowIndexKey {
    buffer_version: u64,
    fold_version: u64,
    visible_lines: usize,
    wrap_enabled: bool,
    wrap_column: Option<usize>,
    viewport_width_bits: u32,
    glyph_width_bits: u32,
}

/// MT-054 PERF CAP (adversarial-review hardening): a cached prefix-sum of per-visible-line visual-row
/// counts under word wrap, so the paint path never re-wraps the whole post-fold document every frame.
///
/// `cumulative[i]` is the total number of visual rows produced by visible lines `0..i` (so
/// `cumulative[0] == 0` and `cumulative[visible_lines] == total_rows`). Given a visual-row index `v`, a
/// binary search over `cumulative` yields the visible-line slot that owns it in O(log visible_lines),
/// and the per-line wrap fragments for only the lines intersecting the painted window are materialized
/// lazily by [`CodeEditorPanel::wrap_rows_for_window`] (O(painted window), NOT O(document)).
///
/// The index is rebuilt only on a [`WrapRowIndexKey`] miss (edit / fold / toggle / resize / metric
/// change), so a scroll / hover / idle repaint is a cache hit and costs O(1) for the scroll-row count.
struct WrapRowIndex {
    key: WrapRowIndexKey,
    /// `cumulative.len() == visible_lines + 1`; `cumulative[i]` = visual rows in visible lines `0..i`.
    cumulative: Vec<usize>,
}

impl WrapRowIndex {
    /// Total visual rows across the whole visible (post-fold) document — the `show_rows` row count.
    fn total_rows(&self) -> usize {
        *self.cumulative.last().unwrap_or(&0)
    }

    /// The visible-line slot (index into the fold-mapped visible window) that owns visual-row index `v`,
    /// plus the visual-row index at which that visible line's fragments begin. Returns `None` when `v` is
    /// past the end. O(log visible_lines).
    fn visible_line_for_row(&self, v: usize) -> Option<(usize, usize)> {
        if v >= self.total_rows() {
            return None;
        }
        // `cumulative` is sorted nondecreasing; find the last slot whose start is <= v.
        // partition_point returns the number of leading elements with start <= v, so subtract 1.
        let slot = self.cumulative.partition_point(|&start| start <= v) - 1;
        Some((slot, self.cumulative[slot]))
    }
}

#[derive(Clone)]
struct PendingCodeTextUndo {
    before: TextBuffer,
    after: TextBuffer,
    description: &'static str,
    replace_tail: bool,
}

#[derive(Debug)]
struct PendingCodeEditMutationReceipt {
    line_delta: i64,
    pane_id: Option<PaneId>,
    workspace_id: String,
    file_path: String,
}

#[derive(Default)]
struct CodeTextUndoBatcher {
    batch_before: Option<TextBuffer>,
    last_edit_at: Option<Instant>,
}

impl CodeTextUndoBatcher {
    fn observe_edit(&mut self, before: TextBuffer, now: Instant) -> (TextBuffer, bool) {
        let replace_tail = self
            .last_edit_at
            .is_some_and(|last| now.duration_since(last) <= CODE_TEXT_UNDO_BATCH_WINDOW);
        if replace_tail {
            let batch_before = self.batch_before.clone().unwrap_or_else(|| before.clone());
            self.batch_before = Some(batch_before.clone());
            self.last_edit_at = Some(now);
            (batch_before, true)
        } else {
            self.batch_before = Some(before.clone());
            self.last_edit_at = Some(now);
            (before, false)
        }
    }

    fn reset(&mut self) {
        self.batch_before = None;
        self.last_edit_at = None;
    }
}

/// The native code-editor panel widget. Holds the document buffer + highlighter and renders the
/// visible lines as colored runs, virtualized through `ScrollArea::show_rows` (MT-002).
///
/// ## Why `Mutex`/atomic interior mutability rather than `RefCell`
///
/// [`CodeEditorPaneFactory`] holds the panel behind an `Arc` and the WP-011 `PaneFactory` trait is
/// `Send + Sync`, so the panel must be `Sync`. `RefCell`/`Cell` are not `Sync`; the mutable
/// render-side state therefore lives behind `Mutex`/atomics. The panel still renders on the single
/// egui UI thread, so contention is nil — the locks exist only to satisfy the `Sync` bound the shell
/// trait requires (no fork of the trait, per the MT "reuse the WP-011 shell, do not fork" rule).
pub struct CodeEditorPanel {
    /// The document buffer behind a `Mutex` so an input-driven edit (`insert_at_all` /
    /// `delete_at_all`) can mutate it through the `&self` render path while the panel stays `Sync`
    /// (the `Arc`-held `PaneFactory` requirement). Reads lock briefly; the egui UI thread is the only
    /// accessor so contention is nil. (MT-003: edits land now, so the buffer is no longer immutable.)
    buffer: Mutex<TextBuffer>,
    /// `None` when the document's extension has no registered grammar (plain text, no highlighting).
    highlighter: Mutex<Option<Highlighter>>,
    /// Monotonic version counter bumped on every buffer-mutating operation (edits land in MT-003).
    /// The highlight cache is valid only while it matches `buffer_version` — this is the invalidation
    /// hook that must also be bumped on undo/redo so a length-changing undo cannot leave stale spans
    /// (RISK-002). Atomic so a `&self` edit/refresh can bump it under the `Sync` panel.
    buffer_version: AtomicU64,
    /// Buffer version last known to match durable storage. `load_file` establishes the baseline and
    /// a successful host save advances it. Keeping this on the document model prevents the shell from
    /// accidentally treating an edit made before the first rendered frame as the clean baseline.
    saved_buffer_version: AtomicU64,
    /// Process-monotonic host identity. Unlike an `Arc` address this cannot be reused after close,
    /// so delayed save completions cannot ABA-match a reopened panel allocation.
    host_incarnation: u64,
    /// Cached highlight spans + the `buffer_version` they were computed for (MT-002 step 3). Recomputed
    /// only when the version changes, so the render path never re-parses every frame.
    highlight_cache: Mutex<Option<(HighlightSpanWindow, u64)>>,
    /// Large initial documents complete their full parse synchronously. Their immutable source and a
    /// clone of that completed tree are queued only after construction, so no second parse or worker
    /// contention can precede the first highlighted-range emission.
    initial_highlight_job: Mutex<Option<InitialHighlightJob>>,
    initial_highlight_rx: Mutex<Option<mpsc::Receiver<InitialHighlightDelivery>>>,
    #[allow(clippy::type_complexity)]
    initial_highlight_source: Mutex<Option<(Arc<[u8]>, u64, u64)>>,
    initial_highlight_cancel: Mutex<Option<Arc<std::sync::atomic::AtomicBool>>>,
    initial_highlight_generation: AtomicU64,
    initial_highlight_attempt: AtomicU8,
    initial_highlight_status: AtomicU8,
    initial_highlight_failure: Mutex<Option<InitialHighlightFailure>>,
    initial_highlight_poll: Mutex<()>,
    #[cfg(test)]
    initial_highlight_test_fault: Mutex<Option<InitialHighlightTestFault>>,
    /// Immutable count captured before a background delivery can replace the cache. Performance proof
    /// code uses this instead of a polling cache read, eliminating the old first/full-span race.
    initial_span_count: usize,
    /// Cached measured monospace line height (px), set on the first `show` from the row height of the
    /// EXACT live [`Self::mono_font`] galley the glyphs are painted with (MT-054 row-pitch
    /// unit fix — one unit for painted rows, `show_rows` stride, gutter, overlays, and decorations).
    /// `None` until measured.
    line_height_px: Mutex<Option<f32>>,
    /// Per-frame virtualization diagnostics (MT-002 step 4), updated each `show`.
    perf: Mutex<PerfStats>,
    /// The line index range painted on the most recent frame — the exact `row_range`
    /// `egui::ScrollArea::show_rows` passed to the paint closure (AC-007), so tests/agents can assert
    /// exactly which lines are on screen (AC-003) and MT-003+ can position the cursor/gutter/selection
    /// overlay against the real painted window. egui applies NO overscan, so this equals the on-screen
    /// rows, not a padded estimate. `0..0` before the first render.
    last_visible_range: Mutex<std::ops::Range<usize>>,
    /// The live vertical scroll offset in pixels from egui's own `ScrollArea` state, updated each
    /// render. This preserves fractional/partial-row scroll for overlays that must align to pixels.
    last_scroll_offset_px: Mutex<f32>,
    /// A one-shot requested vertical scroll offset (px from content top). When set, the next `show`
    /// forces the `ScrollArea` to that offset via `vertical_scroll_offset` and clears the request, so
    /// a caller (a go-to-line action in a later MT, a swarm agent, or a deterministic test) can scroll
    /// the editor to a known position without reaching into egui's persisted scroll state.
    pending_scroll_offset: Mutex<Option<f32>>,
    /// A line-target request made before the first render has measured the exact row height. The next
    /// `show` converts it to pixels with the live measured metric, then consumes it through the same
    /// one-shot `pending_scroll_offset` path. This is required for cross-file definition navigation:
    /// newly mounted documents are navigated before their first frame.
    pending_scroll_line: Mutex<Option<usize>>,
    /// One-shot focus request used by cross-surface navigation. The next mounted render transfers
    /// keyboard and AccessKit focus to the real code text node, then clears the request.
    editor_focus_pending: std::sync::atomic::AtomicBool,
    /// Instance discriminator for AccessKit author_ids (RISK-004). Empty for the default single panel
    /// so it uses the bare MT-contract ids.
    instance: String,
    /// MT-003 multi-cursor + selection state. The single owner of editing intent; the render path
    /// reads it to paint carets/selections and to emit the `code_editor_cursor_{n}` AccessKit nodes,
    /// and the input handler mutates it (Alt+Click, Ctrl+Alt+Up/Down, Alt+Shift drag, Ctrl+D). Behind
    /// a `Mutex` for the same `Sync` reason as the buffer.
    cursor_set: Mutex<CursorSet>,
    /// The `(line, col)` where an Alt+Shift box-selection drag began, or `None` when no box drag is in
    /// progress (MT-003 step 5). Stored in line/column units so the column range is computed directly.
    box_drag_start: Mutex<Option<(usize, usize)>>,
    /// Cached monospace glyph width (px) for column<->x mapping in the caret/selection overlay, measured
    /// once with the SAME live [`Self::mono_font`] the glyphs are painted with so a caret at
    /// column `c` lands exactly on column `c`'s glyph (MT-003 positioning requirement). `None` until
    /// measured on the first `show`.
    glyph_width_px: Mutex<Option<f32>>,
    /// The screen-space geometry of the most recent painted row window, captured inside `render_rows`
    /// so the caret/selection overlay and pointer hit-testing share egui's ACTUAL layout (no separate
    /// recompute). `None` before the first render.
    row_geometry: Mutex<Option<RowGeometry>>,
    /// The exact gutter row model the body paint path produced this frame. This is distinct from
    /// `last_visible_range`: under word wrap that range is VISUAL-row space, so reconstructing buffer
    /// lines from it would put gutter line 1 beside a continuation fragment of line 0.
    last_gutter_paint_rows: Mutex<Vec<GutterPaintRow>>,
    /// MT-004 in-file find/replace state. `None` when the find bar is closed (no highlights painted —
    /// AC-006); `Some` while it is open. The find bar UI reads + mutates it; `process_find_input`
    /// opens/closes it on Ctrl+F / Ctrl+H / Escape. Behind a `Mutex` for the same `Sync` reason as the
    /// buffer.
    find_state: Mutex<Option<FindState>>,
    /// MT-108 (MT-004 residual): set true when the find bar is opened so `render_find_bar` requests
    /// keyboard focus on the find input on the FIRST frame after opening (VS Code auto-focuses the find
    /// box on Ctrl+F), then swap-clears it so focus is not stolen every frame. This is also what lets a
    /// kittest type real characters into the find TextEdit.
    find_focus_pending: std::sync::atomic::AtomicBool,
    /// True while either native find/replace `TextEdit` owns keyboard focus. The code editor is a
    /// custom-painted surface, so its input loop sees the same global egui events as the focused
    /// `TextEdit`; this fence prevents query typing, IME commits, Backspace/Delete, arrows, and other
    /// text-field keys from also mutating the code buffer.
    find_text_input_focused: std::sync::atomic::AtomicBool,
    /// MT-005 code-folding state: the fold regions derived from the tree-sitter parse tree plus their
    /// folded flags. Recomputed only when `buffer_version` changes (MT impl note 3 — tracked by
    /// `fold_version`), then carried across frames so a user's collapsed regions stay collapsed. Behind
    /// a `Mutex` for the same `Sync` reason as the buffer.
    fold_set: Mutex<FoldSet>,
    /// The `buffer_version` the fold regions were last computed for. When it lags `buffer_version` the
    /// next `show` recomputes the regions from the highlighter's current tree (MT impl note 3); on a
    /// match the fold regions are reused (no re-walk every frame). `0` until the first computation.
    fold_version: AtomicU64,
    /// The stable language-family id (`"rust"` / `"javascript"`, or `""` when unmapped) used to select
    /// folding's foldable-node table (MT-005). Captured at build time from the document extension so
    /// the fold provider does not re-derive it every frame.
    language_id: &'static str,
    /// The document's file extension (lowercased), captured at build time so a structural buffer
    /// replacement (MT-051 line transforms) can rebuild a FRESH highlighter for the same grammar. The
    /// tree-sitter highlighter re-parses incrementally from its cached tree (highlight.rs); a transform
    /// that replaces whole lines without an `InputEdit` would leave that cached tree describing offsets
    /// past the new (shorter) buffer and panic on re-highlight. Rebuilding the highlighter resets its
    /// incremental state to a clean FULL parse (see `reset_highlighter`).
    extension: String,
    /// MT-006 outline (symbol tree) cache: the symbols extracted from the SAME tree-sitter tree the
    /// highlighter built (no second parse — MC-002), recomputed only when the buffer version moves
    /// (tracked by `outline_version`). Behind a `Mutex` for the same `Sync` reason as the buffer.
    outline_items: Mutex<Vec<OutlineItem>>,
    /// The `buffer_version` the outline was last computed for. When it lags `buffer_version` the next
    /// access recomputes the outline from the highlighter's current tree (MC-002). `0` until first
    /// computed.
    outline_version: AtomicU64,
    /// One-shot default-visibility decision deferred until the first outline derivation. Buffer load
    /// returns as soon as tree-sitter has emitted syntax ranges; fold/outline tree walks are independent
    /// consumers and must not delay that first usable editor state.
    outline_default_pending: std::sync::atomic::AtomicBool,
    /// MT-006: whether the outline side panel is shown (RISK-001 / MC-001 — hideable so the center
    /// editor keeps a usable width). Default ON for a language with symbols; the toggle button + the
    /// `set_show_outline` API flip it. Atomic so the `&self` render path / agent can flip it.
    show_outline: std::sync::atomic::AtomicBool,
    /// MT-006: whether the minimap side panel is shown (RISK-001 / MC-001 — hideable). Default ON; the
    /// toggle button + `set_show_minimap` flip it.
    show_minimap: std::sync::atomic::AtomicBool,
    /// MT-006 go-to-line palette state. `None` when the palette is closed (no modal, no AccessKit node);
    /// `Some` while it is open (Ctrl+G). Behind a `Mutex` for the same `Sync` reason as the buffer.
    goto_line_state: Mutex<Option<GotoLineState>>,
    /// MT-053 in-file Go to Symbol palette (Ctrl+Shift+O). The file-scoped quick-outline, sourced by
    /// flattening the MT-006 outline (no re-parse). Closed by default (no modal, no AccessKit node).
    /// Behind a `Mutex` for the same `Sync` reason as the buffer. STRICTLY DISTINCT from the global
    /// MT-030 quick-switcher (different palette, different data scope).
    symbol_palette: Mutex<super::symbol_palette::SymbolPalette>,
    /// MT-053 sticky-scroll computer (its config: max pinned headers). Stateless apart from the config;
    /// the pinned headers are recomputed every frame from the current scroll offset + the live MT-005
    /// fold regions (no caching across edits — RISK-004 / MC-004).
    sticky_scroll: super::sticky_scroll::StickyScroll,
    /// MT-006 minimap widget (its configured width). Stateless apart from the width; carried so the
    /// width can be tuned without re-threading it through `show`.
    minimap: Minimap,
    /// The screen rect the minimap occupied on the most recent frame (diagnostics + the deterministic
    /// midpoint-click test — AC-006). `None` before the first render or while the minimap is hidden.
    last_minimap_rect: Mutex<Option<egui::Rect>>,
    /// The screen rect the outline panel occupied on the most recent frame (diagnostics + the
    /// three-panel layout test — AC-003). `None` before the first render or while the outline is hidden.
    last_outline_rect: Mutex<Option<egui::Rect>>,
    /// Cached minimap per-row colors + the `(buffer_version, painted_rows, dark_mode, syntax_palette)` key
    /// they were computed for. The minimap's only O(spans) pass ([`Minimap::compute_row_colors`]) runs ONLY
    /// on a cache miss (buffer edit, panel resize, theme flip, or Custom palette edit), so the per-frame minimap render is
    /// O(painted_rows) — critical on a 100k-line file where re-walking every span each frame blows the
    /// MT-002 frame budget. `None` until the first minimap render.
    minimap_row_cache: Mutex<Option<MinimapRowCache>>,
    /// MT-007 gutter feature flags (line numbers / fold triangles / diagnostics / breakpoints). Behind
    /// a `Mutex` so a settings change / agent can flip a column under the `Sync` panel. Defaults all-on.
    gutter_config: Mutex<GutterConfig>,
    /// MT-007 breakpoint state: the buffer lines that carry a breakpoint. The gutter draws a red circle
    /// per line here, a gutter click toggles it, and a toggle publishes a `BreakpointEvent`. Behind a
    /// `Mutex` for the same `Sync` reason as the buffer.
    breakpoint_set: Mutex<BreakpointSet>,
    /// MT-007 diagnostic markers populated by MT-008's LSP client via [`push_diagnostics`]. Starts
    /// EMPTY (this MT prepares the slot). Stored in INDEPENDENT state with NO `buffer_version` bump
    /// (KERNEL_BUILDER gate: a diagnostics push must NOT trigger the MT-002 highlight-cache / tree
    /// re-parse — see `push_diagnostics`). The gutter reads this to draw severity dots + left bars.
    diagnostic_markers: Mutex<Vec<GutterMarker>>,
    /// MT-046 IC-09: related-note destinations attached to diagnostic lines. These are rendered as
    /// actual clickable gutter chips and exposed through stable AccessKit ids; a click dispatches the
    /// canonical `open-document` command on the shared InteractionBus.
    diagnostic_note_references: Mutex<std::collections::BTreeMap<usize, String>>,
    /// MT-052 jump-history stack (Navigate Back / Forward — Alt+Left / Alt+Right). In-memory SESSION
    /// state only (no PostgreSQL/EventLedger persistence — the MT is pure frontend). It records the
    /// PRE-jump cursor location at the four navigation-jump dispatch sites (goto-def / references /
    /// outline / goto-line) so Navigate Back can restore it, including across files. Behind a `Mutex` for
    /// the same `Sync` reason as the buffer.
    jump_history: Mutex<JumpHistory>,
    /// MT-052 pending CROSS-FILE jump target: set when a Navigate Back/Forward restores a position in a
    /// file OTHER than the one this panel currently shows. The actual document swap is the E11 host-mount
    /// MT's job, so MT-052 parks the intent here (instead of moving the caret in the wrong file —
    /// RISK-005) and the host drains it. `None` when the last restore was same-file or none happened.
    pending_cross_file_jump: Mutex<Option<JumpEntry>>,
    /// Pane currently rendering this shared document. Host-routed events capture it at emission time
    /// so duplicate tabs in docked/pop-out panes never infer origin from document identity alone.
    host_render_pane_id: Mutex<Option<PaneId>>,
    pending_cross_file_jump_origin: Mutex<Option<PaneId>>,
    /// MT-007 breakpoint publish channel to the FUTURE debug-adapter (DAP) client. The sender is held
    /// here (cloned for each publish); the receiver is held until a DAP client takes it via
    /// [`subscribe_breakpoints`]. An UNBOUNDED `std::sync::mpsc` channel + `send().ok()` is the
    /// non-blocking, discard-on-disconnect publish the MT red-team RISK-003 wants (std `Sender` has no
    /// `try_send`; that is `SyncSender` on a bounded channel — KERNEL_BUILDER gate resolution).
    breakpoint_sender: mpsc::Sender<BreakpointEvent>,
    /// The receive half of the breakpoint channel, taken (once) by the future DAP client via
    /// [`subscribe_breakpoints`]. Held here so the channel is not closed before a subscriber exists
    /// (publishes are then a benign no-op — RISK-003). `None` after a subscriber takes it.
    breakpoint_receiver: Mutex<Option<mpsc::Receiver<BreakpointEvent>>>,
    /// The path of the file this panel edits, carried on every published `BreakpointEvent` so the DAP
    /// client can map breakpoints to a source. Empty for an in-memory buffer. Set via
    /// [`set_file_path`] / cleared+seeded by [`load_file`].
    file_path: Mutex<String>,
    /// The screen rect the gutter strip occupied on the most recent frame (diagnostics + the
    /// deterministic gutter-click test — AC-005/AC-006). `None` before the first render.
    last_gutter_rect: Mutex<Option<egui::Rect>>,
    /// The buffer line of each PAINTED gutter row, in painted order, captured on the last frame so a
    /// test can compute the exact pixel to click for a known line (the gutter aligns to these rows). The
    /// gutter geometry (origin/line_height/char_width) it was painted at is in `last_gutter_geometry`.
    last_gutter_rows: Mutex<Vec<usize>>,
    /// The gutter geometry of the most recent frame (origin/line_height/char_width), so a test can map a
    /// painted gutter row index to its screen y. `None` before the first render.
    last_gutter_geometry: Mutex<Option<GutterGeometry>>,

    // ── MT-008 code intelligence (LSP + Handshake code-nav fallback) ──────────────────────────────
    /// MT-008 completion popup state. `None` when no completion is showing; `Some` while the popup is
    /// open. The render path draws the popup + emits its AccessKit nodes from this; the input handler
    /// (Arrow/Enter/Escape) and the result-delivery drain mutate it. Behind a `Mutex` for the same
    /// `Sync` reason as the buffer.
    completion_state: Mutex<Option<CompletionState>>,
    /// Durable acknowledgement state for transient completion rows. Every newly opened popup resets
    /// this observer to a fresh Ready generation. Only a successful click outcome advances it to the
    /// matching Applied generation; keyboard acceptance deliberately does not claim a Click receipt.
    completion_observer: Mutex<CompletionObserverState>,
    completion_visible_identity: Mutex<Option<CodeIntelligenceRequestIdentity>>,
    /// MT-008 hover tooltip state. `None` when no hover is showing; `Some` while the tooltip is open.
    hover_state: Mutex<Option<HoverState>>,
    hover_visible_identity: Mutex<Option<CodeIntelligenceRequestIdentity>>,
    /// MT-008 Handshake backend code-nav client (the fallback intelligence source). Reused for
    /// completion + hover + go-to-def + references when no LSP server is attached. Cheap to clone.
    code_nav_client: Mutex<CodeNavClient>,
    /// MT-008 short-lived `lookup_symbols(prefix)` cache (RISK-002 / MC-004 — debounce + cache).
    code_nav_cache: Mutex<CodeNavCache>,
    /// MT-008 LSP client (lazily spawns a language server on first `did_open`). Defaults to
    /// [`LspClient::disabled`] (graceful empty results — AC-004) until a server is configured. Behind a
    /// `Mutex` so the `&self` render/input path can drive it under the `Sync` panel; an `Arc` so the
    /// off-thread completion/hover task can hold it across an await.
    lsp_client: Mutex<Arc<LspClient>>,
    /// Monotonic identity of the newest completion request. Every trigger attempt increments it,
    /// including attempts that short-circuit, so a response for an older prefix can never become the
    /// current popup after the caret/prefix has moved on.
    completion_generation: AtomicU64,
    /// Monotonic identity of the newest hover request. Kept separate from completion so the two
    /// independent overlays do not invalidate one another.
    hover_generation: AtomicU64,
    /// Monotonic identities for F12 and Shift+F12. Navigation results are rejected when their request
    /// generation, buffer, caret, document, or workspace no longer matches the live panel.
    definition_generation: AtomicU64,
    references_generation: Arc<AtomicU64>,
    /// MT-008 active workspace id used for the backend code-nav lookups (empty = no workspace bound,
    /// so code-nav requests are skipped — the React `activeWorkspaceId() == null` short-circuit).
    workspace_id: Mutex<String>,
    /// MT-008 instant of the last buffer edit (implementation note 2). The completion trigger only
    /// fires when this is at least [`COMPLETION_DEBOUNCE_MS`] in the past, so fast typing does not flood
    /// the backend (RISK-002). `None` until the first edit.
    last_edit_instant: Mutex<Option<std::time::Instant>>,
    /// MT-008 hover-dwell tracker (implementation note 3): the `(cursor_byte_offset, since, fired)` the
    /// cursor has rested at. A hover request fires once per settled offset after the dwell exceeds
    /// [`HOVER_DWELL_MS`], preventing repeated backend lookups every frame while the caret is parked.
    /// `None` when no dwell is in progress.
    hover_dwell: Mutex<Option<(usize, std::time::Instant, bool)>>,
    /// MT-008 off-thread completion result delivery cell. The LSP-first/fallback task writes a
    /// generation-tagged delivery here; the next `show` validates it before updating `completion_state`
    /// (HBR-QUIET — the egui thread never blocks on either intelligence source).
    completion_result: CompletionResultCell,
    /// MT-008 off-thread hover result delivery cell. The LSP-first/fallback task writes a
    /// generation-tagged delivery here; the next `show` validates it before updating `hover_state`.
    hover_result: HoverResultCell,
    /// MT-008 off-thread code-nav symbol delivery queue used by the explicit raw-result injection seam.
    /// Live completion/hover fallback batches travel with their generation-tagged deliveries so stale
    /// requests cannot mutate the cache or staleness gutter.
    code_nav_symbols_result: CodeNavSymbolsResultCell,
    /// MT-010 off-thread go-to-definition result cell (F12). A spawned `lookup_symbols` task writes the
    /// resolved 0-based definition line here; the next `show` drains it and calls `navigate_to_line`.
    /// Reuses the MT-008 code-nav client + the MT-006 line-navigation path (no new backend surface).
    goto_def_result: GotoDefResultCell,
    /// MT-010 off-thread references result cell (Shift+F12). LSP and CodeNav deliveries are normalized
    /// into the same actionable overlay on the next frame.
    references_result: ReferencesResultCell,
    /// MT-010 the most recent CodeNav ShowReferences result (callers + callees), exposed alongside the
    /// normalized overlay for backend-specific inspection.
    last_references: Mutex<Option<CodeSymbolReferencesResponse>>,
    /// Most recent LSP definition/references, retaining URI + range instead of discarding cross-file
    /// identity or flattening locations to a line number.
    last_definition_target: Mutex<Option<CodeNavigationLocation>>,
    last_lsp_references: Mutex<Vec<CodeNavigationLocation>>,
    reference_items: Mutex<Vec<CodeReferenceItem>>,
    references_visible_identity: Mutex<Option<CodeIntelligenceRequestIdentity>>,
    /// Independent broadcast subscription for this panel. Shared LSP clients therefore fan diagnostics
    /// out to every open pane instead of handing the stream to whichever pane drains first.
    lsp_diagnostics_rx: Mutex<Option<tokio::sync::broadcast::Receiver<PublishedDiagnostics>>>,
    /// Last accepted version per current document URI. This prevents a delayed same-document diagnostic
    /// set from replacing markers for a newer local buffer generation.
    lsp_diagnostics_version: Mutex<Option<(String, i64)>>,
    /// MT-008 the app's tokio runtime handle, injected by the host (the same per-component injection
    /// pattern `BackendClient`/`ProjectTree`/`QuickSwitcher` use — see [`set_runtime`](Self::set_runtime)).
    /// The LIVE render/input loop reads it to drive the off-thread completion/hover triggers from
    /// `show()`/`process_cursor_input` (HBR-QUIET — the egui thread never blocks; the triggers `spawn`
    /// onto this handle). `None` until the host injects one; while `None` the live code-intelligence
    /// loop is a graceful no-op (the synthetic `open_completion`/`open_hover` test paths still work), so
    /// a runtime-less unit/kittest harness renders without spawning backend tasks.
    runtime: Mutex<Option<tokio::runtime::Handle>>,
    /// MT-008 "a completion request is armed this frame" flag. `process_cursor_input` sets it on
    /// Ctrl+Space or a completion trigger character (`.`/`:`/`_`); the per-frame
    /// [`pump_code_intelligence`](Self::pump_code_intelligence) consumes it (take + reset) and fires the
    /// debounced backend completion lookup. An atomic so the `&self` input path can arm it under the
    /// `Sync` panel without holding a lock across the input loop.
    completion_request: AtomicU8,
    automatic_completion_cursor: Mutex<Option<usize>>,

    // ── MT-047 signature help (parameter hints) ───────────────────────────────────────────────────
    /// MT-047 the live signature-help popup state. `None` when no popup is open; `Some` while showing.
    /// The render path draws the popup + emits its AccessKit node from this; the input handler (trigger
    /// characters / Ctrl+Shift+Space / dismissal) and the off-thread result drain mutate it. Behind a
    /// `Mutex` for the same `Sync` reason as the buffer.
    signature_help_state: Mutex<Option<SignatureHelpState>>,
    /// MT-047 off-thread signature-help result delivery cell. A spawned LSP-then-code-nav task writes the
    /// resolved [`SignatureHelpState`] here; the next frame's drain swaps it into `signature_help_state`
    /// (HBR-QUIET — the egui thread never blocks on the LSP/backend; the MT-008 delivery-cell shape).
    /// `Arc<Mutex<..>>` so the spawned task + the UI thread share it.
    signature_help_result: SignatureHelpResultCell,
    /// MT-047 "a signature-help request is armed this frame" flag, set by `process_cursor_input` on a
    /// `(`/`,` trigger character or the Ctrl+Shift+Space manual shortcut; consumed (take + reset) by the
    /// per-frame `pump_code_intelligence` which fires the off-thread LSP-then-fallback request. An atomic
    /// so the `&self` input path arms it under the `Sync` panel without holding a lock across the input
    /// loop (the same shape as `completion_request`).
    signature_help_request: std::sync::atomic::AtomicBool,
    /// MT-047 cached fallback signature per `(call-target identifier, open_paren_byte)` for the popup
    /// lifetime (RISK-002 / MC-002), so re-triggering on each comma in the same call does NOT re-hit
    /// `/knowledge/code/symbols`. `None` until the first fallback resolves. An `Arc<Mutex<..>>` so the
    /// off-thread resolve task writes the freshly-resolved symbol straight into it (the UI thread reads
    /// it on the next trigger).
    signature_fallback_cache: SignatureFallbackCache,
    /// MT-047 (AC-002 dismissal) whether the code TEXT surface currently holds focus, mirrored each frame
    /// from the pane factory's `has_focus` (`ui.memory().focused()` == the pane's egui id) BEFORE `show()`
    /// runs. The per-frame signature-help dismissal guard closes the popup when this is `false` (the editor
    /// lost focus — scope step 8), alongside the caret-left-the-call check. Defaults to `true` so a headless
    /// harness that drives `show()` directly (no factory — e.g. the synthetic render/AccessKit proofs) is
    /// never spuriously dismissed; the live factory path and the interaction tests set it explicitly via
    /// [`set_code_surface_focus`](Self::set_code_surface_focus).
    code_surface_focused: std::sync::atomic::AtomicBool,

    // ── MT-048 Rename Symbol (F2) ─────────────────────────────────────────────────────────────────
    /// MT-048 the rename state machine phase (Idle / Editing the inline input / Previewing the multi-file
    /// WorkspaceEdit / Error). The render path draws the input/preview/banner from this; the F2 keymap, the
    /// context-menu entry, and the off-thread rename result drain mutate it. Behind a `Mutex` for the same
    /// `Sync` reason as the buffer.
    rename_state: Mutex<RenameState>,
    /// MT-048 off-thread rename result delivery cell: a spawned LSP-`textDocument/rename`-then-fallback
    /// task writes the resolved [`WorkspaceEditPreview`] (or an error message) here; the next frame's
    /// drain swaps it into `rename_state::Previewing`/`Error` (HBR-QUIET — the egui thread never blocks on
    /// the LSP/backend; the MT-008 delivery-cell shape). `Arc<Mutex<..>>` so the spawned task + the UI
    /// thread share it.
    rename_result: RenameResultCell,

    // ── MT-049 Code actions / quick fixes (the lightbulb) ─────────────────────────────────────────
    /// MT-049 the quick-fix controller: owns the code-action request lifecycle, the action-list + menu
    /// state, the gutter-lightbulb decision, and the apply call (which DELEGATES to the MT-048 apply path).
    /// The cursor-rest trigger + Ctrl+. + the context-menu 'Quick Fix...' entry feed it; the render path
    /// draws the lightbulb + menu from it. Behind a `Mutex` for the same `Sync` reason as the buffer.
    code_action_controller: Mutex<CodeActionController>,
    /// Actual quick-fix lightbulb draw positions from the most recent frame, exposed only for kittest
    /// regression proofs that need to compare gutter chrome against body row centers.
    last_quickfix_lightbulbs: Mutex<Vec<(usize, egui::Pos2)>>,
    /// MT-049 off-thread code-action result delivery cell: a spawned `textDocument/codeAction` task sends
    /// the resolved [`CodeActionResult`] over this channel; [`CodeActionController::poll_results`] drains it
    /// each frame (HBR-QUIET — the egui thread never blocks on the LSP; the MT-008 off-thread pattern). The
    /// sender is cloned into each spawned request; the receiver is installed on the controller once.
    code_action_tx: mpsc::Sender<code_actions::CodeActionResult>,
    /// MT-049 the result receiver, parked here until [`pump_code_actions`](Self::pump_code_actions)
    /// installs it on the controller on the first frame (one consumer per channel). `None` after install.
    code_action_rx: Mutex<Option<mpsc::Receiver<code_actions::CodeActionResult>>>,
    /// MT-049 the cursor-rest debounce: the `(line, since)` the cursor has rested on. A code-action request
    /// fires once the rest exceeds the debounce window AND the line carries >=1 diagnostic (RISK-001 /
    /// MC-001 — never per idle frame; cancel on a line change). `None` when the cursor is moving / off a
    /// diagnostic line. Behind a `Mutex` for the same `Sync` reason as the buffer.
    code_action_rest: Mutex<Option<(usize, std::time::Instant)>>,
    /// MT-049 the cursor-rest debounce threshold (default [`CODE_ACTION_REST_MS`]ms). A kittest sets it to
    /// ZERO so the rest crossing fires on the first settled frame, driving the REAL cursor-rest pipeline
    /// deterministically WITHOUT a wall-clock wait. Behind a `Mutex` for the `Sync` panel.
    code_action_rest_threshold: Mutex<std::time::Duration>,
    /// MT-049 one-shot Ctrl+. (or context-menu) arm: set by the keymap dispatch / context-menu entry, drained
    /// by the per-frame pump which fires the code-action request AND opens the menu immediately (vs the
    /// passive cursor-rest path that only lights the bulb). Atomic so the `&self` dispatch can arm it.
    quick_fix_request: std::sync::atomic::AtomicBool,
    /// Monotonic count of real quick-fix requests entering [`Self::trigger_quick_fix`]. Unlike the
    /// one-shot arm, this cannot be consumed before canonical post-state inspection.
    quick_fix_request_generation: std::sync::atomic::AtomicU64,
    /// Last concrete request tuple `(line, buffer_version, open_menu)` captured at the handler boundary.
    last_quick_fix_request: Mutex<Option<(usize, u64, bool)>>,
    /// MT-049 the LAST cross-file quick-fix apply outcome (RISK-005 / MC-005). When a chosen code action's
    /// `WorkspaceEdit` touches files OTHER than the active buffer, [`apply_quickfix`](Self::apply_quickfix)
    /// routes them through MT-048's [`rename::apply_preview`] (atomic to-disk write) and records the
    /// `Result<RenameApplyReport, String>` here — `Ok` with the files/edits applied, or `Err` with the
    /// `RenameError` message (e.g. a missing/locked target file). MC-005 requires the cross-file outcome be
    /// SURFACED + logged, never silently dropped; this cell is the typed, queryable surface (a `tracing`
    /// warn/info is emitted alongside) so the failure path is observable to a swarm agent + a unit test even
    /// when the in-file edit already committed. `None` until the first cross-file apply. Behind a `Mutex`
    /// for the same `Sync` reason as the buffer.
    last_quickfix_cross_file: Mutex<Option<Result<RenameApplyReport, String>>>,

    // ── MT-050 Format Document / Format Selection ─────────────────────────────────────────────────
    /// MT-050 one-shot Alt+Shift+F (or EDIT-menu / context-menu 'Format Document') arm: set by the keymap
    /// dispatch / menu entry, drained by the per-frame pump which fires the `textDocument/formatting`
    /// request off-thread and applies the returned TextEdits as one undo step. Atomic so the `&self`
    /// dispatch can arm it. A no-op when no formatter is available (the disabled keymap path — AC-003).
    format_document_request: std::sync::atomic::AtomicBool,
    /// MT-050 one-shot 'Format Selection' arm (context-menu / AccessKit node). Same off-thread pump path,
    /// issuing `textDocument/rangeFormatting` for the current selection (empty selection -> current line).
    format_selection_request: std::sync::atomic::AtomicBool,
    /// MT-050 off-thread format result delivery cell: a spawned format task writes the resolved
    /// [`FormatOutcome`] here; the next frame's drain installs the formatted text (single undo) + surfaces a
    /// non-blocking toast on the error path (HBR-QUIET — the egui thread never blocks on the LSP). The
    /// payload also carries the pre-format snapshot + the formatted text so the drain can record the single
    /// undo entry on the UI thread (the off-thread task does not touch the buffer). `Arc<Mutex<..>>` so the
    /// spawned task + the UI thread share it.
    format_result: FormatResultCell,
    /// MT-050 the LAST format toast (the non-blocking LspError / NoFormatter surface — AC-006). Queryable by
    /// a swarm agent + a unit test; `None` until the first non-applied format outcome that warrants a toast.
    last_format_toast: Mutex<Option<String>>,
    /// MT-050 the queued single-undo snapshot `(before_text, after_text)` for a just-applied format. The
    /// panel records it on the UI thread (in `drain_format_result`); the factory render drains it into
    /// `interop_adapter::push_code_edit_undo` so ONE undo entry is recorded at the bus boundary (AC-001).
    pending_format_undo: Mutex<Option<(String, String)>>,

    // ── MT-046 Copy as note reference (code -> note interconnection edge) ─────────────────────────
    /// MT-046 the queued `[[code:…]]` note-reference string a 'Copy as note reference' dispatch built
    /// from the current selection / identifier. The factory render drains it via
    /// [`take_pending_copy_note_reference`](Self::take_pending_copy_note_reference) and writes it to the
    /// SHARED InteractionBus clipboard (`interop_adapter::copy_note_reference_to_bus`) — the panel has
    /// no bus handle of its own, so the staged string is the typed hand-off (the same pattern the
    /// pending format/line-op undo snapshots use).
    pending_copy_note_reference: Mutex<Option<String>>,
    /// MT-070/MT-057 the queued create-note-from-link intent: the `[[title]]` under the cursor when the
    /// editor-body context menu's 'Create note from link' entry was confirmed. The host/shell drains it
    /// via [`take_pending_create_note_link`](Self::take_pending_create_note_link) and routes it to the
    /// MT-057 create-note intent handler (the code panel itself has no wikilink runtime).
    pending_create_note_link: Mutex<Option<String>>,
    /// A snapshot of the mounted rich editor's authoritative wikilink resolver. `None` means the
    /// workspace enumeration has not completed successfully, so Create-note-from-link fails closed.
    wikilink_resolver_index: Mutex<Option<crate::rich_editor::wikilinks::resolver::ResolverIndex>>,
    /// Canonical snapshot capture runs on a fresh egui context. Retain whether the operator/agent opened
    /// this panel's real editor-body menu so that fresh Argus inspection can reproduce that dynamic
    /// popup without inventing an alternate action surface.
    context_menu_open_for_snapshot: std::sync::atomic::AtomicBool,
    snapshot_capture_mode: std::sync::atomic::AtomicBool,

    // ── MT-051 line-edit buffer transforms ────────────────────────────────────────────────────────
    /// MT-051 the queued single-undo snapshot `(description, before_text, after_text)` for a just-applied
    /// line transform (ToggleComment / DuplicateLine / MoveLine / DeleteLine / Indent / Dedent / InsertTab).
    /// Each `line_ops` transform snapshots the whole buffer before + after and queues ONE entry here; the
    /// factory render drains it into `interop_adapter::push_code_edit_undo` so a single Ctrl+Z reverts the
    /// whole transform (RISK-003 / AC-007) — the SAME bus boundary every code edit's undo is recorded at
    /// (the MT-035/050 wrap-not-fork pattern; no parallel undo stack). Only the latest is kept (a second
    /// transform before the drain supersedes; the host applies them in order so the newest pair is correct).
    pending_line_op_undo: Mutex<Option<(&'static str, String, String)>>,
    /// MT-035 the queued live text-edit undo snapshot `(before, after)` for Event::Text / IME commit /
    /// newline / Backspace / Delete. The panel stages rope snapshots here and the factory render drains
    /// them into the shared `InteractionBus`, preserving the single unified undo authority.
    pending_text_edit_undo: Mutex<Option<PendingCodeTextUndo>>,
    /// MT-036/MT-069 code-edit producer receipt. Every successful product mutation adds its exact line
    /// delta here; the mounted factory drains the accumulated delta into the existing two-second
    /// trailing-edge Flight Recorder batch. A zero delta is meaningful (an in-line edit still emits), while
    /// failed, cancelled, and byte-for-byte no-op paths never stage a receipt.
    pending_code_edit_receipts: Mutex<VecDeque<PendingCodeEditMutationReceipt>>,
    /// MT-035 code-side typing batcher. Edits inside the same 500ms burst replace the pane's local undo
    /// tail so one Ctrl+Z reverts to the first pre-burst snapshot instead of stepping per character.
    text_edit_undo_batcher: Mutex<CodeTextUndoBatcher>,
    /// MT-051 the operator's `editor.tabSize` (one indent unit = this many spaces when `insert_spaces`).
    /// Sourced from the editor-settings layer via [`set_indent_settings`](Self::set_indent_settings);
    /// defaults to VS Code's 4. Atomic so the `&self` dispatch reads it without locking. Never hardcoded
    /// at a `line_ops` call site (MC-006).
    tab_size: AtomicU64,
    /// MT-051 the operator's `editor.insertSpaces`: when true one indent unit is `tab_size` spaces, when
    /// false it is a literal tab (RISK-006 / MC-006). Defaults to VS Code's true. Atomic for `&self` reads.
    insert_spaces: std::sync::atomic::AtomicBool,

    // ── MT-071 file-metadata state (status-bar segments: language / EOL / encoding / whitespace) ───
    //
    // These hang OFF the doc model (RISK-004/MC-004) so they survive re-render + re-focus and the
    // MT-001 draw + the language resolver read them. (Indent lives in the existing tab_size/insert_spaces
    // above, REUSED — the Indent segment drives those.)
    /// MT-071 the per-document USER language override (the highest-precedence detection layer). `None`
    /// while the language is auto-detected; `Some(family_id)` once the user picks one from the status-bar
    /// language picker. Read by [`resolved_language`](Self::resolved_language) (override beats shebang /
    /// content / extension — RISK-003) and persists across re-render + re-focus (RISK-004).
    language_override: Mutex<Option<super::language_mode::LanguageId>>,
    /// MT-071 (perf, adversarial-review must-fix #4) the cached resolved language keyed on
    /// `(buffer_version, override)`. [`resolved_language`](Self::resolved_language) is called every frame
    /// the status bar renders; without a cache it would `buffer().to_string()` the WHOLE document each
    /// frame (an O(buffer) copy that scales with file size). The cache recomputes only when the buffer
    /// version bumps (an edit) or the user override changes, so an idle frame is a cheap version+override
    /// compare. `None` until the first resolve.
    resolved_language_cache: Mutex<
        Option<(
            u64,
            Option<super::language_mode::LanguageId>,
            super::language_mode::LanguageDetection,
        )>,
    >,
    /// MT-071 the document's active line-ending style (LF / CRLF). Seeded from the buffer on build
    /// ([`Eol::detect`](super::file_meta::Eol::detect)); the status-bar EOL segment + its "Convert to
    /// LF/CRLF" actions read + set it. Behind a `Mutex` for the same `Sync` reason as the buffer.
    eol: Mutex<super::file_meta::Eol>,
    /// MT-071 the document's active text encoding (default UTF-8). The status-bar encoding segment shows
    /// it and "Reopen with Encoding" re-decodes the on-disk bytes under the chosen encoding through the
    /// MT-010 load path (no backend call — RISK-005).
    encoding: Mutex<super::file_meta::Encoding>,
    /// MT-071 the render-whitespace toggle. `true` makes the MT-001 editor DRAW path render middots for
    /// spaces + arrows for tabs; the status-bar whitespace segment flips it. Atomic so the `&self` draw
    /// path / an agent reads it without locking.
    render_whitespace: std::sync::atomic::AtomicBool,
    /// WP-KERNEL-012 MT-035 (settings completeness): the LIVE render-whitespace MODE (0=None, 1=Boundary,
    /// 2=All) the shell threads in from `editor_prefs.render_whitespace`. It supersedes the boolean lossiness
    /// (Boundary and All previously both collapsed to `true`): `paint_whitespace_glyphs` reads THIS to skip
    /// single inter-word spaces in Boundary mode (VS Code parity). Kept in lockstep with the bool
    /// `render_whitespace` (mode != None <=> bool true) so the status-bar toggle path still works. Atomic so
    /// the `&self` draw path / an agent reads it without locking.
    render_whitespace_mode: std::sync::atomic::AtomicU8,
    /// WP-KERNEL-012 MT-035 (settings completeness): whether the sticky-scroll pinned-header band renders.
    /// Default `true` (the feature was always-on before). The shell threads `editor_prefs.sticky_scroll` in
    /// via [`set_sticky_scroll_enabled`](Self::set_sticky_scroll_enabled); `render_sticky_band` early-returns
    /// when `false`. Atomic so the `&self` draw path reads it without locking.
    sticky_scroll_enabled: std::sync::atomic::AtomicBool,
    /// WP-KERNEL-012 wave-6 (S6 item 3 / the MT-072 font-size follow-up): the LIVE editor font size (pt)
    /// the shell threads in from `editor_prefs.editor_font_size` via
    /// [`set_font_size`](Self::set_font_size). `None` = use the built-in [`MONO_FONT_SIZE`] default. The
    /// measurement (`line_height` / `glyph_width`) AND every panel-body glyph-paint site read it through
    /// [`mono_font`](Self::mono_font)/[`font_size`](Self::font_size), so a settings change resizes the
    /// running editor (row height + glyphs) with no restart. Behind a `Mutex` for the same `Sync` reason
    /// as the buffer; `set_font_size` clears the measured-metric caches so the next frame re-measures.
    font_size: Mutex<Option<f32>>,
    /// WP-KERNEL-012 wave-6 (S6 item 3 / the MT-072 syntax-palette follow-up): the LIVE Custom syntax
    /// palette the shell threads in from `syntax_palette` via
    /// [`set_syntax_palette`](Self::set_syntax_palette). `None` (or a non-`Custom` mode) keeps the
    /// theme-driven [`scope_to_color`]; a `Custom` palette routes every highlight-run color through the
    /// LIVE [`resolve_scope_color`](crate::code_editor::resolve_scope_color) resolver, so a Custom swatch
    /// edit repaints the running editor. Behind a `Mutex` for the same `Sync` reason as the buffer.
    syntax_palette: Mutex<Option<crate::workspace_settings::SyntaxPalette>>,
    /// WP-KERNEL-012 MT-035 wave-7: the LIVE row-height MULTIPLIER the shell threads in from
    /// `editor_prefs.line_height` via [`set_line_height`](Self::set_line_height). `None` = `1.0`
    /// (single-spaced — the font's natural measured row height). [`line_height`](Self::line_height)
    /// multiplies the measured mono-font row height by this, so every stride/decoration/overlay that
    /// derives from `line_height` spaces uniformly; `set_line_height` clears the `line_height_px` cache so
    /// the next frame re-measures. Behind a `Mutex` for the same `Sync` reason as the buffer.
    line_height_multiplier: Mutex<Option<f32>>,
    /// WP-KERNEL-012 MT-035 wave-7: whether the matching-bracket highlight renders (the shell threads
    /// `editor_prefs.bracket_matching` in via
    /// [`set_bracket_matching_enabled`](Self::set_bracket_matching_enabled)). Default `true` (the highlight
    /// was always on before). When `false`, [`matching_bracket_at`](Self::matching_bracket_at) returns
    /// `None` and `paint_chrome_decorations` skips the matched-bracket box. Atomic so the `&self` draw path
    /// reads it without locking.
    bracket_matching_enabled: std::sync::atomic::AtomicBool,
    /// WP-KERNEL-012 MT-035 wave-7: whether vertical indent-guide lines render (the shell threads
    /// `editor_prefs.indent_guides` in via [`set_indent_guides_enabled`](Self::set_indent_guides_enabled)).
    /// Default `true` (indent guides were always on before). When `false`, `paint_chrome_decorations`
    /// skips the indent-guide pass and [`indent_guide_count_for_line`](Self::indent_guide_count_for_line)
    /// reports `0`. Atomic so the `&self` draw path reads it without locking.
    indent_guides_enabled: std::sync::atomic::AtomicBool,

    // ── MT-054 editor chrome: word wrap + bracket match/colorize + indent guides ──────────────────
    /// MT-054 the word-wrap configuration (Alt+Z). `enabled == false` by default (the MT-002 baseline
    /// 1:1 render — RISK-006 / MC-006). The `show` path consumes the Alt+Z shortcut to flip `enabled`,
    /// refreshes `viewport_width_px` each frame from the live editor-area width, and drives BOTH the
    /// `show_rows` row count + scroll math AND the per-row paint from the resulting VisualRow list
    /// (RISK-001 / MC-001 — one source of truth). Behind a `Mutex` for the same `Sync` reason as the
    /// buffer; a swarm agent flips it via the `editor-wrap-toggle` AccessKit node.
    wrap_config: Mutex<WrapConfig>,
    /// MT-072 Fix 3 (MT-054 wrap-persistence closeout): set to `true` by [`toggle_wrap`](Self::toggle_wrap)
    /// — the single mutation point Alt+Z, the visible "Wrap" button, and the `editor-wrap-toggle` AccessKit
    /// node all route through — to signal a USER-initiated wrap change. The host drains it once per frame
    /// via [`take_user_wrap_toggle`](Self::take_user_wrap_toggle) and writes the new state back into the
    /// persisted `editor_prefs.word_wrap` (so Alt+Z persists across restart). A prefs->panel
    /// [`set_wrap_enabled`](Self::set_wrap_enabled) push does NOT set this flag, so the write-back is a
    /// one-way user->prefs signal that never ping-pongs.
    wrap_toggled_by_user: std::sync::atomic::AtomicBool,
    /// MT-054 PERF CAP (adversarial-review hardening): the cached wrap-row COUNT index that lets the paint
    /// path compute `show_rows`' total visual-row count + map a visual-row index back to its visible line
    /// WITHOUT re-wrapping the whole post-fold document every frame. Recomputed only when its key changes
    /// (buffer edit, wrap toggle / column / viewport-width change, glyph-width change, or fold-state
    /// change) — NOT on a scroll / hover / idle repaint. On a cache hit the per-frame scroll-count lookup
    /// is O(1) and the per-frame paint materializes ONLY the logical lines intersecting the painted visual
    /// row window (O(window), not O(document)). `None` until the first wrap frame builds it.
    wrap_row_index: Mutex<Option<WrapRowIndex>>,

    // ── MT-010 Monaco-parity keymap (the SINGLE key dispatch authority) ───────────────────────────
    /// MT-010 the active keymap: the VS Code default binding table merged with any operator overrides
    /// loaded from `~/.handshake/keymap.json`. The SINGLE source of truth for "what does this key do" —
    /// `process_keymap` resolves every editor key event through this table and dispatches the resolved
    /// [`CodeEditorAction`]. Behind a `Mutex` so a hot-reload (the override file changed) can swap the
    /// table in under the `Sync` panel. Bumps `keymap_version` on every swap so the cached AccessKit
    /// command nodes + chord hints rebuild (RISK-002 caching).
    keymap: Mutex<Keymap>,
    /// MT-010 monotonic version bumped on every keymap swap (override reload). The cached command-node
    /// AccessKit set + any chord-hint cache key off this so they rebuild only when the keymap changes,
    /// not every frame (RISK-002).
    keymap_version: AtomicU64,
    /// MT-010 two-chord pending state (RISK-001 / MC-001): the prefix chord (e.g. Ctrl+K) seen but not
    /// yet completed, plus the instant it was seen so a stale prefix clears after
    /// [`crate::code_editor::panel`] `TWO_CHORD_TIMEOUT`. `None` when no prefix is pending. Behind a
    /// `Mutex` for the same `Sync` reason as the buffer.
    pending_chord: Mutex<Option<(KeyChord, std::time::Instant)>>,
    /// MT-010 the resolved `~/.handshake/keymap.json` override-file path (via `dirs::home_dir()` — AC-007,
    /// no hardcoded path), captured once at build so the per-frame hot-reload poll does not re-resolve it.
    /// `None` when the home directory is unresolvable (headless/sandboxed) — the reload poll is then a
    /// graceful no-op and the editor uses the in-memory keymap.
    keymap_file_path: Option<std::path::PathBuf>,
    /// MT-010 the last-seen mtime of the override file + the instant it was last polled. The per-frame
    /// `maybe_reload_keymap` stats the file at most once per [`KEYMAP_RELOAD_POLL_SECS`]; when the mtime
    /// moves it reloads the keymap from disk (implementation note 6). `None` mtime until the first poll.
    keymap_file_state: Mutex<(Option<std::time::SystemTime>, Option<std::time::Instant>)>,
    /// MT-010 optional command-palette dispatch channel (implementation note: `OpenCommandPalette` routes
    /// to the SAME WP-011 command palette the rest of the shell uses — `command_palette.rs` backed by
    /// `command_registry.rs` — via an `mpsc::Sender` the host injects, NOT a second palette). `None` when
    /// no host wired a palette (the action is then a graceful no-op + a trace), so a headless test panel
    /// renders without a palette. Behind a `Mutex` for the `Sync` panel.
    command_palette_tx: Mutex<Option<(mpsc::Sender<CodeEditorHostCommand>, String)>>,
    /// MT-010 cached AccessKit command-node descriptors + the `keymap_version` they were built for
    /// (RISK-002 / MC-004 — build the 56-node set ONCE per keymap change, NOT every frame). The render
    /// path reads this cache to emit the hidden `Role::Button` command nodes;
    /// [`ensure_command_nodes`](Self::ensure_command_nodes) rebuilds it only on a version miss. `None`
    /// until the first emit.
    command_node_cache: Mutex<Option<(u64, Vec<CommandNodeDesc>)>>,
    /// WP-KERNEL-012 MT-041 (E7): the consolidated editor-action AccessKit surface wiring. `None` until
    /// the host (or a kittest) installs a shared [`EditorActionRegistry`] via
    /// [`install_editor_action_registry`](Self::install_editor_action_registry). When installed, each
    /// `show` registers/updates this pane's canonical `editor.code.<action>` nodes in the shared registry,
    /// emits them into the live tree, and consumes any swarm `Action::Click` dispatched at them — the
    /// single swarm-facing action surface that CONSOLIDATES (does not re-mint) the per-MT widget nodes.
    /// Behind a `Mutex` for the `Sync` panel.
    editor_action_wiring: Mutex<Option<EditorActionWiring>>,

    // ── MT-034 code->notes cross-references (the NoteRefsPanel side surface) ───────────────────────────
    /// MT-034: whether the "Notes referencing this symbol" panel is shown in the right sidebar
    /// (RISK-001 / MC-001 — hideable so the center editor keeps a usable width, like the outline/minimap).
    /// Default OFF (it loads only on a symbol dwell; an empty panel adds nothing but width until then —
    /// the operator toggles it on). Atomic so the `&self` render path / an agent can flip it.
    show_note_refs: std::sync::atomic::AtomicBool,
    /// MT-034: the async load state of the NoteRefsPanel for the currently-dwelled symbol. The render
    /// path reads it; [`pump_note_refs`](Self::pump_note_refs) sets it to `Loading` when it fires a
    /// search and the drain swaps in the delivered `Loaded`/`Failed` result. Behind a `Mutex` for the
    /// same `Sync` reason as the buffer.
    note_refs_state: Mutex<NoteRefsState>,
    /// MT-034: the 800ms cursor-dwell debounce (RISK-3 / MC-3 — fire the notes search ONCE per dwell, never
    /// per cursor move / per frame). [`pump_note_refs`](Self::pump_note_refs) calls `observe` each frame
    /// with the word under the caret; a dwell crossing fires the off-thread search. Behind a `Mutex` for
    /// the same `Sync` reason as the buffer.
    note_refs_dwell: Mutex<SymbolDwellTracker>,
    /// MT-034: the symbol KEY the NoteRefsPanel last loaded/loads for (the panel header text + the search
    /// query). `None` until the first dwell. Distinct from the raw caret word — it is the resolved
    /// `symbol_key` from `lookup_symbols` (the precise multi-token `path#Symbol` that cuts false positives
    /// — RISK-1). Behind a `Mutex` for the same `Sync` reason as the buffer.
    note_refs_focused_symbol: Mutex<Option<String>>,
    /// MT-034 off-thread find-notes result delivery cell: a spawned `find_notes_referencing_symbol` task
    /// writes the resolved [`NoteRefsState`] (`Loaded`/`Failed`) here; the next frame's drain swaps it into
    /// `note_refs_state` (HBR-QUIET — the egui thread never blocks on the backend; the MT-008 delivery-cell
    /// shape). `Arc<Mutex<..>>` so the spawned task + the UI thread share it.
    note_refs_result: NoteRefsResultCell,
    /// Exact active request identity. A delivery must match all fields before it can update the panel.
    note_refs_active_request: Mutex<Option<NoteRefsRequestStamp>>,
    /// Monotonic ownership generation for NoteRefs requests.
    note_refs_generation: AtomicU64,
    /// Last workspace/file/cursor context observed by the dwell pump. Any change invalidates in-flight
    /// work immediately, including a move within the same identifier.
    note_refs_observed_context: Mutex<Option<(String, String, usize)>>,
    /// MT-034 the find-notes search backend (injectable so a kittest drives the live dwell->search->panel
    /// path with a counted in-memory mock and NO backend, the MT-014/MT-015 fetcher-trait pattern). The
    /// production default is [`FindNotesHttp`] (the verified `POST /workspaces/{ws}/loom/search-v2` route).
    /// `Arc` so the off-thread spawn can hold it across an await. Behind a `Mutex` so a test can inject a
    /// mock under the `Sync` panel.
    find_notes_backend: Mutex<Arc<dyn FindNotesSearch>>,
    /// MT-034 the cursor-dwell threshold the live `pump_note_refs` uses (default
    /// [`crate::interop::NOTE_REFS_DWELL_MS`]ms). A kittest sets it to ZERO via
    /// [`set_note_refs_dwell_threshold`](Self::set_note_refs_dwell_threshold) so the dwell crossing fires
    /// on the first settled frame, driving the REAL dwell->search->panel pipeline deterministically
    /// WITHOUT an 800ms wall-clock wait. Behind a `Mutex` for the `Sync` panel.
    note_refs_dwell_threshold: Mutex<std::time::Duration>,
    /// A clicked NoteRefs document retained until it is successfully staged on the shared bus.
    pending_note_ref_open: Mutex<Option<String>>,
    /// WP-KERNEL-012 MT-076 (E13 IME inline preedit): the IN-PROGRESS IME composition (preedit) text for
    /// the code editor, shown UNDERLINED at the primary caret. Empty when no composition is active. This
    /// is OVERLAY-ONLY — it is NEVER written into the buffer (RISK-1 / MC-1: only `Event::Ime::Commit`
    /// inserts, via the proven char-correct `insert_text` path); painting it preserves the same
    /// double-insert invariant the rich editor uses. Behind a `Mutex` for the `Sync` panel (the `&self`
    /// input path mutates it; the `&self` render path reads it).
    preedit: Mutex<String>,
    /// WP-KERNEL-012 MT-080 (AC-080-6 / MT-043 swarm-authoring): the LIVE `egui::Id` of the text-input node
    /// recorded each frame by the render path (the node is emitted on `ui.unique_id()` inside the text
    /// scope, NOT on `text_id()`, so the swarm-action consumer must read action requests at this exact id —
    /// reading `text_id()` would never match the dispatched `AccessKitActionRequest::target`). `None` before
    /// the first render. Behind a `Mutex` for the `Sync` panel (the `&self` render path writes it; the
    /// `&self` `consume_swarm_text_actions` reads it).
    live_text_node_id: Mutex<Option<egui::Id>>,
    /// MT-108: the stable find-bar AccessKit node id recorded each frame so a queued Argus SetValue
    /// action can update the real find query through the existing panel setter (rather than a synthetic
    /// test-only shortcut).
    live_find_node_id: Mutex<Option<egui::Id>>,
}

/// MT-034 off-thread find-notes result delivery cell: the resolved [`NoteRefsState`] written by a spawned
/// `find_notes_referencing_symbol` task and drained on the next frame into `note_refs_state`. Aliased so
/// the panel field type stays legible (clippy `type_complexity`).
#[derive(Clone, Debug, PartialEq, Eq)]
struct NoteRefsRequestStamp {
    workspace_id: String,
    symbol: String,
    generation: u64,
}

#[derive(Clone, Debug)]
struct NoteRefsDelivery {
    stamp: NoteRefsRequestStamp,
    symbol_key: Option<String>,
    state: NoteRefsState,
}

type NoteRefsResultCell = Arc<Mutex<Vec<NoteRefsDelivery>>>;

/// MT-010 one cached AccessKit command-node descriptor: the fixed `node_id`, the `code_editor_cmd_*`
/// author_id, the chord-annotated label, and the action it dispatches. Built once per keymap version
/// (RISK-002) and reused across frames so a 56-action editor does not recompute 56 nodes every frame.
#[derive(Clone, Debug)]
struct CommandNodeDesc {
    /// The `egui::Id` the node is emitted onto (default panel: a fixed id in the command band; instance:
    /// a hashed id from the suffixed author_id — RISK-004). `accesskit_node_builder` keys on this id.
    node_id: egui::Id,
    /// The `code_editor_cmd_{action_name}` author_id a swarm agent / MCP tool addresses.
    author_id: String,
    /// The human label (description + the bound chord, e.g. "Find (Ctrl+F)").
    label: String,
    /// The action this node dispatches when activated.
    action: CodeEditorAction,
}

/// WP-KERNEL-012 MT-041 (E7): the installed editor-action AccessKit wiring for a code pane — the shared
/// [`EditorActionRegistry`] this pane writes its canonical `editor.code.<action>` nodes into, plus the
/// [`RegistrationHandle`] carrying its stable instance index (RISK-041-05).
struct EditorActionWiring {
    registry: Arc<Mutex<EditorActionRegistry>>,
    handle: RegistrationHandle,
}

/// MT-010 two-chord timeout (RISK-001 / MC-001 / AC-002): if the second chord of a two-chord binding
/// (e.g. Ctrl+K then Ctrl+0) does not arrive within this window, the pending prefix is cleared and no
/// action fires, so a stale Ctrl+K never wedges single-chord shortcuts. The contract names 3 seconds.
pub const TWO_CHORD_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

/// MT-010 three-state outcome of context-sensitive key resolution (step 3). The `Consumed` state is the
/// load-bearing distinction from a plain `Option`: a goto-line Enter SUBMIT must be `Consumed` so the
/// keymap does NOT also resolve Enter to `InsertNewline` and type a stray newline.
enum ContextOutcome {
    /// Resolve to this state-specific action (the dispatcher runs it).
    Dispatch(CodeEditorAction),
    /// The key was handled here; do nothing further this event (do NOT fall through to the binding).
    Consumed,
    /// No contextual override applies; fall through to the plain single-chord binding.
    FallThrough,
}

/// Identity captured when an asynchronous completion/hover request is issued. The UI drain validates
/// this against the live panel before applying a response, which rejects results for an older request,
/// buffer snapshot, caret, document, or workspace.
#[derive(Clone, Debug, PartialEq, Eq)]
struct CodeIntelligenceRequestIdentity {
    generation: u64,
    buffer_version: u64,
    cursor_byte: usize,
    document_uri: Option<String>,
    workspace_id: String,
    query: String,
}

/// A code-nav fallback batch travels with the response that produced it. This keeps stale response
/// batches from updating the cache or gutter after their request identity is no longer current.
type CodeNavFallbackBatch = (String, Vec<CodeSymbolNavProjection>);

#[derive(Debug)]
struct CompletionDelivery {
    request: CodeIntelligenceRequestIdentity,
    anchor: egui::Pos2,
    items: Vec<CompletionItem>,
    code_nav_batch: Option<CodeNavFallbackBatch>,
}

#[derive(Debug)]
struct HoverDelivery {
    request: CodeIntelligenceRequestIdentity,
    hover: Option<HoverState>,
    code_nav_batch: Option<CodeNavFallbackBatch>,
}

#[derive(Debug)]
struct GotoDefinitionDelivery {
    request: CodeIntelligenceRequestIdentity,
    target: Option<CodeNavigationLocation>,
    origin_pane: Option<PaneId>,
}

#[derive(Debug)]
enum ReferencesPayload {
    Lsp(Vec<CodeNavigationLocation>),
    CodeNav {
        raw: CodeSymbolReferencesResponse,
        items: Vec<CodeReferenceItem>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CodeReferenceItem {
    label: String,
    target: CodeNavigationLocation,
}

#[derive(Debug)]
struct ReferencesDelivery {
    request: CodeIntelligenceRequestIdentity,
    payload: ReferencesPayload,
}

/// MT-008 off-thread completion result delivery cell. The generation-aware replacement rule prevents
/// an older task that finishes last from overwriting a newer undrained response.
type CompletionResultCell = Arc<Mutex<Option<CompletionDelivery>>>;

/// MT-008 off-thread hover result delivery cell, with the same generation-aware replacement rule.
type HoverResultCell = Arc<Mutex<Option<HoverDelivery>>>;

/// MT-008 explicit raw code-nav symbol injection queue, drained on the next frame for staleness markers
/// and cache. Workspace identity is captured at enqueue time so a later workspace switch cannot attribute
/// the batch to the wrong cache key. Live fallback batches travel with their request identity.
type CodeNavSymbolsResultCell = Arc<Mutex<Vec<(String, String, Vec<CodeSymbolNavProjection>)>>>;

/// MT-010 off-thread go-to-definition result cell: the 0-based target buffer line written by a spawned
/// `lookup_symbols` task (F12 / GoToDefinition) and drained on the next frame to `navigate_to_line`.
type GotoDefResultCell = Arc<Mutex<Option<GotoDefinitionDelivery>>>;

/// MT-010 off-thread references result cell. LSP locations and CodeNav callers/callees are normalized
/// into the same actionable reference items before the next UI-frame drain.
type ReferencesResultCell = Arc<Mutex<Option<ReferencesDelivery>>>;

const COMPLETION_REQUEST_NONE: u8 = 0;
const COMPLETION_REQUEST_AUTOMATIC: u8 = 1;
const COMPLETION_REQUEST_EXPLICIT: u8 = 2;

fn decode_percent_encoded_path(path: &str) -> Option<std::path::PathBuf> {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            let hex = |byte: u8| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                b'A'..=b'F' => Some(byte - b'A' + 10),
                _ => None,
            };
            decoded.push((hex(high)? << 4) | hex(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded).ok()?;
    Some(std::path::PathBuf::from(
        decoded.replace('/', std::path::MAIN_SEPARATOR_STR),
    ))
}

fn path_from_lsp_uri(uri: &lsp_types::Url) -> Option<std::path::PathBuf> {
    uri.to_file_path().ok().or_else(|| {
        // A platform-neutral server may emit `file:///sibling.rs`. Windows cannot convert that URI to
        // an absolute path because it has no drive letter. Preserve the decoded relative path; the
        // navigation application step anchors it against the current document directory.
        (uri.scheme() == "file")
            .then(|| uri.path().trim_start_matches('/'))
            .filter(|path| !path.is_empty())
            .and_then(decode_percent_encoded_path)
    })
}

fn normalized_path_key(path: &std::path::Path) -> String {
    let mut key = path.to_string_lossy().replace('\\', "/");
    while key.contains("//") {
        key = key.replace("//", "/");
    }
    #[cfg(windows)]
    {
        key.make_ascii_lowercase();
    }
    key
}

fn same_lsp_document_uri(expected: &str, actual: &str) -> bool {
    if expected == actual {
        return true;
    }
    let Some(expected) = lsp_types::Url::parse(expected).ok() else {
        return false;
    };
    let Some(actual) = lsp_types::Url::parse(actual).ok() else {
        return false;
    };
    match (path_from_lsp_uri(&expected), path_from_lsp_uri(&actual)) {
        (Some(expected), Some(actual)) => {
            normalized_path_key(&expected) == normalized_path_key(&actual)
        }
        _ => expected == actual,
    }
}

fn navigation_location_from_lsp(location: lsp_types::Location) -> CodeNavigationLocation {
    let path = path_from_lsp_uri(&location.uri).map(|path| path.to_string_lossy().to_string());
    CodeNavigationLocation {
        uri: location.uri.to_string(),
        path,
        range: location.range,
    }
}

fn code_nav_location_from_symbol(
    symbol: &CodeSymbolNavProjection,
    current_file_path: &str,
) -> Option<CodeNavigationLocation> {
    let definition = symbol.definition.as_ref()?;
    let start_line = definition.line_start?.checked_sub(1)? as u32;
    let end_line = definition
        .line_end
        .and_then(|line| line.checked_sub(1))
        .unwrap_or(start_line as i64) as u32;
    // The symbol key owns the target file. `source_id` is only a fallback for older projections whose
    // key did not embed a path; never substitute the currently open document URI.
    let target_path = symbol_file_path(&symbol.symbol_key).or_else(|| {
        definition
            .source_id
            .as_ref()
            .filter(|source| !source.trim().is_empty())
            .cloned()
    })?;
    let target = std::path::PathBuf::from(&target_path);
    let current = std::path::PathBuf::from(current_file_path);
    let resolved = if target.is_absolute() {
        target
    } else if !current_file_path.trim().is_empty() && current.ends_with(&target) {
        current
    } else {
        current
            .parent()
            .into_iter()
            .flat_map(|parent| parent.ancestors())
            .map(|ancestor| ancestor.join(&target))
            .find(|candidate| candidate.exists())
            .unwrap_or(target)
    };
    let uri = if resolved.is_absolute() {
        lsp_types::Url::from_file_path(&resolved)
            .ok()
            .map(|uri| uri.to_string())
    } else {
        Some(format!(
            "file:///{}",
            resolved
                .to_string_lossy()
                .replace('\\', "/")
                .trim_start_matches('/')
        ))
    }?;
    Some(CodeNavigationLocation {
        uri,
        path: Some(resolved.to_string_lossy().to_string()),
        range: lsp_types::Range::new(
            lsp_types::Position::new(start_line, 0),
            lsp_types::Position::new(end_line, 0),
        ),
    })
}

/// MT-047 off-thread signature-help result delivery cell: the resolved [`SignatureHelpState`] written by
/// a spawned LSP-then-code-nav task and drained on the next frame into `signature_help_state`. Aliased so
/// the panel field type stays legible (clippy `type_complexity`).
type SignatureHelpResultCell = Arc<Mutex<Option<SignatureHelpState>>>;

/// MT-047 fallback-signature cache: the resolved code-nav symbol keyed by `(call-target identifier,
/// open_paren_byte)` so commas in the same call reuse it instead of re-hitting `/knowledge/code/symbols`
/// (RISK-002 / MC-002). `Arc<Mutex<..>>` so the off-thread resolve task writes it directly.
type SignatureFallbackCache = Arc<Mutex<Option<(String, usize, CodeSymbolNavProjection)>>>;

/// MT-048 off-thread rename result delivery cell: the resolved rename outcome written by a spawned
/// LSP-`textDocument/rename`-then-fallback task and drained on the next frame into `rename_state`. The
/// `Ok(WorkspaceEditPreview)` variant becomes `RenameState::Previewing`; the `Err(message)` variant
/// becomes `RenameState::Error`. Aliased so the panel field type stays legible (clippy `type_complexity`).
type RenameResultCell = Arc<Mutex<Option<Result<WorkspaceEditPreview, String>>>>;

/// MT-050 off-thread format result delivery cell: a spawned `textDocument/formatting` /
/// `textDocument/rangeFormatting` task writes `(pre_format_snapshot, FormatOutcome_with_formatted_text)`
/// here, and the next frame's drain installs the formatted text (recording ONE undo entry on the UI
/// thread) or surfaces the no-formatter / error toast. The `Option<String>` is the formatted text (present
/// only on an `Applied` outcome that changed the buffer); the `FormatOutcome` carries the typed result the
/// drain reports + toasts. Aliased so the field type stays legible (clippy `type_complexity`).
type FormatResultCell = Arc<Mutex<Option<(String, Option<String>, FormatOutcome)>>>;

/// The cached minimap row colors plus the cache key they were computed for: `(colors, buffer_version,
/// painted_rows, dark_mode, syntax_palette)`. Aliased so the `minimap_row_cache` field type stays legible
/// (clippy `type_complexity`).
type MinimapRowCache = (
    Vec<egui::Color32>,
    u64,
    usize,
    bool,
    Option<crate::workspace_settings::SyntaxPalette>,
);

/// MT-006 go-to-line palette state. Owned by [`CodeEditorPanel`] behind a `Mutex`; present only while
/// the palette is open (Ctrl+G). The modal pre-populates `input` with the current cursor line; on Enter
/// the panel parses it, clamps to the buffer, and scrolls. `parsed` caches the last successful parse so
/// the modal can show validity feedback without re-parsing every frame.
#[derive(Clone, Debug, Default)]
pub struct GotoLineState {
    /// The text typed into the go-to-line input (a 1-based line number, as the user sees line numbers).
    pub input: String,
    /// The last successfully-parsed 0-based buffer line from `input`, or `None` when `input` is empty
    /// or not a valid line number (AC-002: non-numeric input parses to `None` -> no navigation, no
    /// crash). Recomputed by [`GotoLineState::reparse`] whenever `input` changes.
    pub parsed: Option<usize>,
}

impl GotoLineState {
    /// Build a state pre-populated with the 1-based form of `cursor_line` (0-based), the VS Code
    /// behavior of seeding the input with the current line.
    fn for_cursor_line(cursor_line: usize) -> Self {
        let one_based = cursor_line.saturating_add(1);
        let mut s = Self {
            input: one_based.to_string(),
            parsed: None,
        };
        s.reparse(usize::MAX); // clamp computed against the live buffer at submit; seed parsed now.
        s
    }

    /// Re-parse `input` into a 0-based buffer line, clamping to `0..len_lines` (RISK-003 / MC-003 —
    /// `0`, negative, and past-the-end inputs clamp without panic; non-numeric inputs yield `None`).
    /// `len_lines` is the live buffer line count (pass `usize::MAX` to defer the clamp to submit time).
    /// Sets + returns `self.parsed`.
    fn reparse(&mut self, len_lines: usize) -> Option<usize> {
        // Parse as i64 so a leading '-' or '0' is handled deterministically (RISK-003). 1-based input.
        let trimmed = self.input.trim();
        self.parsed = match trimmed.parse::<i64>() {
            Ok(n) => {
                // Clamp the 1-based number to 1..=len_lines, then convert to 0-based. n<=0 clamps to
                // line 1 (0-based 0); n>len clamps to the last line.
                let max_one_based = len_lines.min(i64::MAX as usize) as i64;
                let clamped = n.clamp(1, max_one_based.max(1));
                Some((clamped - 1).max(0) as usize)
            }
            Err(_) => None, // non-numeric -> no navigation (AC-002)
        };
        self.parsed
    }
}

/// MT-004 find/replace UI + match state. Owned by [`CodeEditorPanel`] behind a `Mutex`; present only
/// while the find bar is open. Mirrors the React editor's find-panel state (the ported
/// [`FindQuery`] + the match list + the current-match index + the replace text + whether the replace
/// row is shown), with the regex compile error surfaced so an invalid pattern shows a message instead
/// of silently finding nothing (AC-003).
#[derive(Clone, Debug)]
struct ReplaceAllPlan {
    query: FindQuery,
    replacement: String,
    matches: Vec<Match>,
    next_match: usize,
    cumulative_byte_delta: i64,
    expected_buffer_version: u64,
}

#[derive(Clone, Debug, Default)]
pub struct FindState {
    /// The active query (pattern + case/whole-word/regex toggles).
    pub query: FindQuery,
    /// Every match of `query` in the buffer, ascending by byte offset. Recomputed when `query` changes
    /// or after a replace (RISK-003).
    pub matches: Vec<Match>,
    /// Buffer version from which `matches` was computed. Ordinary editor mutations do not eagerly
    /// re-run the find engine, so Replace All checks this value at click time and refreshes before it
    /// builds a new bounded continuation plan.
    matches_buffer_version: u64,
    /// The index into `matches` of the CURRENT match (the one highlighted orange + scrolled to). Always
    /// `< matches.len()` when `matches` is non-empty; clamped on every recompute.
    pub current_match: usize,
    /// The replacement text typed into the replace input (used by Replace / Replace-All).
    pub replace_text: String,
    /// True when the bar is in REPLACE mode (Ctrl+H) — the replace input + buttons are shown.
    pub show_replace: bool,
    /// The regex compile error string for the current `query`, or empty when the pattern compiles / is
    /// not a regex (AC-003: an invalid regex shows this, never panics).
    pub error: String,
    /// MT-108 (MT-004 residual): how many matches a capped Replace All left un-replaced because the set
    /// exceeded [`REPLACE_ALL_CAP`]. 0 when the last Replace All finished the set (the common case) or
    /// none ran. The find bar shows a "N more — click Replace All again" progress hint when > 0. Reset
    /// when the query changes.
    pub replace_all_remaining: usize,
    /// Private continuation state for a capped Replace All. The plan is built from the ORIGINAL match
    /// set and survives the re-search used to refresh highlights after each batch. This is what makes
    /// `x -> x` and `x -> xx` advance to the next original match instead of repeatedly consuming the
    /// first 1000 replacement-generated matches. A buffer-version mismatch invalidates the plan.
    replace_all_plan: Option<ReplaceAllPlan>,
    /// The `query.pattern` value the `matches` were last computed for, so the render loop can detect a
    /// query change (typing in the input) without re-searching every frame.
    last_searched: String,
    /// Whether `last_searched` was computed with these toggle values (so flipping case/whole-word/regex
    /// also triggers a re-search).
    last_toggles: (bool, bool, bool),
}

impl FindState {
    /// The current match (the one highlighted orange + scrolled to), or `None` when there are no
    /// matches.
    pub fn current(&self) -> Option<&Match> {
        self.matches.get(self.current_match)
    }

    /// A human-readable "N of M" counter for the find bar (`0 of 0` when there are no matches; the
    /// current index is 1-based for display).
    pub fn counter_label(&self) -> String {
        if self.matches.is_empty() {
            "0 of 0".to_owned()
        } else {
            format!("{} of {}", self.current_match + 1, self.matches.len())
        }
    }
}

/// Screen-space geometry of the painted row window for one frame (MT-003 overlay positioning). The
/// overlay maps a `(line, col)` to a pixel rect using these: `x = left + col * glyph_width`,
/// `y = top + painted_row_offset * line_height`, where the row offset comes from the FOLD-AWARE
/// painted-lines map (`painted_row_offset` — MT-054 Wave-B fix; `line - first_line` is wrong across a
/// fold). Captured from egui's own row layout so carets align with the glyphs egui actually painted;
/// the rows scope pins its `interact_size.y` floor to `line_height` so the painted pitch IS this unit.
#[derive(Debug, Clone, Copy)]
struct RowGeometry {
    /// Screen x of the left edge of the painted text rows (column 0).
    left: f32,
    /// Screen y of the TOP of the first painted row (`first_line`).
    top: f32,
    /// The line index of the first painted row (`row_range.start`).
    first_line: usize,
    /// Per-row height in px (the sans-spacing line height — same unit `show_rows` strides by).
    line_height: f32,
}

#[derive(Debug, Clone)]
struct CompletionObserverState {
    context: String,
    generation: u64,
    state: ClickCompletionState,
    pending_target: Option<String>,
    semantic_value: Option<String>,
}

impl CompletionObserverState {
    fn ready(context: String, generation: u64) -> Self {
        Self {
            context,
            generation,
            state: ClickCompletionState::Ready,
            pending_target: None,
            semantic_value: None,
        }
    }
}

fn completion_observer_context(instance: &str, workspace_id: &str, file_path: &str) -> String {
    use sha2::{Digest, Sha256};

    let document = if file_path.trim().is_empty() {
        "<in-memory>"
    } else {
        file_path
    };
    let identity = format!("{instance}\0{workspace_id}\0{document}");
    format!(
        "code-editor-document:{:x}",
        Sha256::digest(identity.as_bytes())
    )
}

impl Drop for CodeEditorPanel {
    fn drop(&mut self) {
        if let Ok(cancel_slot) = self.initial_highlight_cancel.get_mut() {
            if let Some(cancel) = cancel_slot.take() {
                cancel.store(true, Ordering::Release);
            }
        }
        if let Ok(job_slot) = self.initial_highlight_job.get_mut() {
            *job_slot = None;
        }
        if let Ok(source_slot) = self.initial_highlight_source.get_mut() {
            *source_slot = None;
        }
    }
}

impl CodeEditorPanel {
    /// Build a panel for `text` with `extension` deciding the grammar (e.g. `"rs"`, `"js"`). An
    /// unknown extension yields a plain (unhighlighted) panel rather than failing.
    pub fn new(text: &str, extension: &str) -> Self {
        Self::build(text, extension, String::new())
    }

    /// Like [`new`](Self::new) but with an `instance` suffix appended to the AccessKit author_ids so
    /// multiple concurrently-mounted panels (e.g. a diff view) stay individually addressable
    /// (RISK-004).
    pub fn with_instance(text: &str, extension: &str, instance: impl Into<String>) -> Self {
        Self::build(text, extension, instance.into())
    }

    fn build(text: &str, extension: &str, instance: String) -> Self {
        let buffer = TextBuffer::new(text);
        let len_lines = buffer.len_lines();
        let registry = LanguageRegistry::with_bundled_languages();
        const LARGE_INITIAL_HIGHLIGHT_LINES: usize = 5_000;
        const FIRST_HIGHLIGHT_WINDOW_LINES: usize = 256;
        let mut highlighter = if len_lines >= LARGE_INITIAL_HIGHLIGHT_LINES {
            registry.initial_highlighter_for_extension(extension)
        } else {
            registry.highlighter_for_extension(extension)
        };
        // Capture the language id from the highlighter (it carries the stable family id), so the fold
        // provider selects the right foldable-node set without re-deriving it every frame (MT-005).
        let language_id = highlighter
            .as_ref()
            .map(|hl| hl.language_id())
            .unwrap_or("");
        // Complete the required full tree-sitter parse and emit syntax ranges before returning from the
        // buffer-load call. Fold regions and outline symbols are independent projections over that SAME
        // cached tree; derive them lazily on first use/show so they cannot delay first highlighted-range
        // availability. Their version 0 sentinels below force exactly one derivation for buffer version 1.
        let mut initial_highlight_job = None;
        let mut initial_highlight_rx = None;
        let mut initial_highlight_source = None;
        let mut initial_highlight_cancel = None;
        let mut initial_highlight_failure = None;
        let spans = if len_lines >= LARGE_INITIAL_HIGHLIGHT_LINES && highlighter.is_some() {
            let window_end = buffer
                .line_to_byte(FIRST_HIGHLIGHT_WINDOW_LINES.min(len_lines))
                .unwrap_or_else(|| buffer.len_bytes());
            let initial = highlighter
                .as_mut()
                .map(|hl| hl.highlight_range(text.as_bytes(), 0..window_end))
                .unwrap_or_default();
            if !initial_highlight_source_is_worker_eligible(text.len()) {
                initial_highlight_failure = Some(InitialHighlightFailure::SourceTooLarge);
            } else if let Some(tree) = highlighter.as_ref().and_then(|hl| hl.tree().cloned()) {
                let source: Arc<[u8]> = Arc::from(text.as_bytes());
                let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
                let (result_tx, result_rx) = mpsc::channel();
                initial_highlight_job = Some(InitialHighlightJob {
                    source: Arc::clone(&source),
                    tree,
                    extension: extension.to_owned(),
                    version: 1,
                    generation: 1,
                    had_initial_spans: !initial.is_empty(),
                    cancel: Arc::clone(&cancel),
                    result_tx,
                    #[cfg(test)]
                    test_fault: None,
                });
                initial_highlight_rx = Some(result_rx);
                initial_highlight_source = Some((source, 1, 1));
                initial_highlight_cancel = Some(cancel);
            } else {
                initial_highlight_failure = Some(InitialHighlightFailure::HighlighterUnavailable);
            }
            initial
        } else {
            highlighter
                .as_mut()
                .map(|hl| hl.highlight(text.as_bytes()))
                .unwrap_or_default()
        };
        let initial_span_count = spans.len();
        let initial_highlight_status = if initial_highlight_job.is_some() {
            INITIAL_HIGHLIGHT_PENDING
        } else if initial_highlight_failure.is_some() {
            INITIAL_HIGHLIGHT_FAILED
        } else {
            INITIAL_HIGHLIGHT_COMPLETE
        };
        let fold_set = FoldSet::new();
        let outline_items = Vec::new();
        let outline_default_pending = highlighter.is_some();
        // MT-007 breakpoint publish channel: unbounded so `send` never blocks; the receiver is parked
        // until a future DAP client subscribes (RISK-003 non-blocking discard-on-disconnect publish).
        let (breakpoint_sender, breakpoint_receiver) = mpsc::channel::<BreakpointEvent>();
        // MT-049 code-action result channel: the sender is cloned into each spawned `textDocument/codeAction`
        // task; the receiver is parked on the panel until the first pump installs it on the controller.
        let (code_action_tx_init, code_action_rx_init) =
            mpsc::channel::<code_actions::CodeActionResult>();
        // MT-071: detect the document's EOL + indent style from its text on open so the status-bar
        // segments + the Tab key reflect the file's real metadata from frame 1 (defaults LF / Spaces 4
        // when ambiguous — MC-007). Pure string analysis, no backend.
        let detected_eol = super::file_meta::Eol::detect(text);
        let detected_indent = super::file_meta::detect_indent(text);
        let initial_completion_observer_context = completion_observer_context(&instance, "", "");
        Self {
            buffer: Mutex::new(buffer),
            highlighter: Mutex::new(highlighter),
            // Version starts at 1 and the initial spans are cached AT version 1, so the first render
            // is a cache hit (no re-parse) and any later edit bumps to 2+ to invalidate.
            buffer_version: AtomicU64::new(1),
            saved_buffer_version: AtomicU64::new(1),
            host_incarnation: CODE_PANEL_INCARNATION_COUNTER
                .fetch_update(
                    std::sync::atomic::Ordering::Relaxed,
                    std::sync::atomic::Ordering::Relaxed,
                    |current| current.checked_add(1),
                )
                .expect("code editor panel incarnation space exhausted"),
            highlight_cache: Mutex::new(Some((HighlightSpanWindow::from_spans(spans), 1))),
            initial_highlight_job: Mutex::new(initial_highlight_job),
            initial_highlight_rx: Mutex::new(initial_highlight_rx),
            initial_highlight_source: Mutex::new(initial_highlight_source),
            initial_highlight_cancel: Mutex::new(initial_highlight_cancel),
            initial_highlight_generation: AtomicU64::new(1),
            initial_highlight_attempt: AtomicU8::new(1),
            initial_highlight_status: AtomicU8::new(initial_highlight_status),
            initial_highlight_failure: Mutex::new(initial_highlight_failure),
            initial_highlight_poll: Mutex::new(()),
            #[cfg(test)]
            initial_highlight_test_fault: Mutex::new(None),
            initial_span_count,
            line_height_px: Mutex::new(None),
            perf: Mutex::new(PerfStats {
                frame_lines_rendered: 0,
                buffer_len_lines: len_lines,
                frame_lines_wrapped: 0,
            }),
            last_visible_range: Mutex::new(0..0),
            last_scroll_offset_px: Mutex::new(0.0),
            pending_scroll_offset: Mutex::new(None),
            pending_scroll_line: Mutex::new(None),
            editor_focus_pending: std::sync::atomic::AtomicBool::new(false),
            find_focus_pending: std::sync::atomic::AtomicBool::new(false),
            find_text_input_focused: std::sync::atomic::AtomicBool::new(false),
            instance,
            cursor_set: Mutex::new(CursorSet::new()),
            box_drag_start: Mutex::new(None),
            glyph_width_px: Mutex::new(None),
            row_geometry: Mutex::new(None),
            last_gutter_paint_rows: Mutex::new(Vec::new()),
            find_state: Mutex::new(None),
            fold_set: Mutex::new(fold_set),
            // Fold/outline projections are intentionally deferred until first use; version 0 forces a
            // tree walk against the already-parsed version-1 tree without a second parse.
            fold_version: AtomicU64::new(0),
            language_id,
            extension: extension.to_ascii_lowercase(),
            outline_items: Mutex::new(outline_items),
            outline_version: AtomicU64::new(0),
            outline_default_pending: std::sync::atomic::AtomicBool::new(outline_default_pending),
            show_outline: std::sync::atomic::AtomicBool::new(false),
            show_minimap: std::sync::atomic::AtomicBool::new(true),
            goto_line_state: Mutex::new(None),
            // MT-053: the in-file symbol palette starts closed; sticky scroll uses the VS Code default
            // (max 5 pinned headers).
            symbol_palette: Mutex::new(super::symbol_palette::SymbolPalette::new()),
            sticky_scroll: super::sticky_scroll::StickyScroll::new(),
            minimap: Minimap::new(),
            last_minimap_rect: Mutex::new(None),
            last_outline_rect: Mutex::new(None),
            minimap_row_cache: Mutex::new(None),
            // MT-007 gutter state. The breakpoint channel is created here (the "bus before producer"
            // shape from the WP-011 event_bus): the sender is held for publishes; the receiver waits
            // for the future DAP client to take it via `subscribe_breakpoints`.
            gutter_config: Mutex::new(GutterConfig::default()),
            breakpoint_set: Mutex::new(BreakpointSet::new()),
            diagnostic_markers: Mutex::new(Vec::new()),
            diagnostic_note_references: Mutex::new(std::collections::BTreeMap::new()),
            jump_history: Mutex::new(JumpHistory::new()),
            pending_cross_file_jump: Mutex::new(None),
            host_render_pane_id: Mutex::new(None),
            pending_cross_file_jump_origin: Mutex::new(None),
            breakpoint_sender,
            breakpoint_receiver: Mutex::new(Some(breakpoint_receiver)),
            file_path: Mutex::new(String::new()),
            last_gutter_rect: Mutex::new(None),
            last_gutter_rows: Mutex::new(Vec::new()),
            last_gutter_geometry: Mutex::new(None),
            // MT-008 code intelligence: the code-nav fallback client + a DISABLED LSP client (graceful
            // empty results until a server is configured — AC-004). No workspace bound yet (code-nav
            // requests are skipped until `set_workspace_id`).
            completion_state: Mutex::new(None),
            completion_observer: Mutex::new(CompletionObserverState::ready(
                initial_completion_observer_context,
                0,
            )),
            completion_visible_identity: Mutex::new(None),
            hover_state: Mutex::new(None),
            hover_visible_identity: Mutex::new(None),
            code_nav_client: Mutex::new(CodeNavClient::production()),
            code_nav_cache: Mutex::new(CodeNavCache::new()),
            lsp_client: Mutex::new(Arc::new(LspClient::disabled())),
            completion_generation: AtomicU64::new(0),
            hover_generation: AtomicU64::new(0),
            definition_generation: AtomicU64::new(0),
            references_generation: Arc::new(AtomicU64::new(0)),
            workspace_id: Mutex::new(String::new()),
            last_edit_instant: Mutex::new(None),
            hover_dwell: Mutex::new(None),
            completion_result: Arc::new(Mutex::new(None)),
            hover_result: Arc::new(Mutex::new(None)),
            code_nav_symbols_result: Arc::new(Mutex::new(Vec::new())),
            goto_def_result: Arc::new(Mutex::new(None)),
            references_result: Arc::new(Mutex::new(None)),
            last_references: Mutex::new(None),
            last_definition_target: Mutex::new(None),
            last_lsp_references: Mutex::new(Vec::new()),
            reference_items: Mutex::new(Vec::new()),
            references_visible_identity: Mutex::new(None),
            lsp_diagnostics_rx: Mutex::new(None),
            lsp_diagnostics_version: Mutex::new(None),
            runtime: Mutex::new(None),
            completion_request: AtomicU8::new(COMPLETION_REQUEST_NONE),
            automatic_completion_cursor: Mutex::new(None),
            // MT-047 signature help: closed until a trigger fires; the delivery cell + request flag start
            // empty; the fallback cache is empty until the first code-nav fallback resolves.
            signature_help_state: Mutex::new(None),
            signature_help_result: Arc::new(Mutex::new(None)),
            signature_help_request: std::sync::atomic::AtomicBool::new(false),
            signature_fallback_cache: Arc::new(Mutex::new(None)),
            // MT-047 (AC-002): assume focused until the factory mirrors real pane focus each frame, so a
            // direct-`show()` harness (no factory) never spuriously dismisses the popup.
            code_surface_focused: std::sync::atomic::AtomicBool::new(true),
            // MT-048 rename: starts Idle; the result cell is empty until an off-thread rename resolves.
            rename_state: Mutex::new(RenameState::Idle),
            rename_result: Arc::new(Mutex::new(None)),
            // MT-049 quick fix: an idle controller; the result channel's receiver is installed on the
            // controller lazily on the first pump (one consumer per channel). The cursor-rest debounce +
            // the Ctrl+. arm start empty; the rest threshold is the ~300ms VS Code lightbulb dwell.
            code_action_controller: Mutex::new(CodeActionController::new()),
            last_quickfix_lightbulbs: Mutex::new(Vec::new()),
            code_action_tx: code_action_tx_init,
            code_action_rx: Mutex::new(Some(code_action_rx_init)),
            code_action_rest: Mutex::new(None),
            code_action_rest_threshold: Mutex::new(std::time::Duration::from_millis(
                CODE_ACTION_REST_MS,
            )),
            quick_fix_request: std::sync::atomic::AtomicBool::new(false),
            quick_fix_request_generation: std::sync::atomic::AtomicU64::new(0),
            last_quick_fix_request: Mutex::new(None),
            // MT-049 cross-file quick-fix outcome surface (RISK-005 / MC-005): empty until the first
            // cross-file apply records its Ok(report)/Err(message) here (never silently dropped).
            last_quickfix_cross_file: Mutex::new(None),
            // MT-050 format: the request arms + the result cell + the toast surface start empty.
            format_document_request: std::sync::atomic::AtomicBool::new(false),
            format_selection_request: std::sync::atomic::AtomicBool::new(false),
            format_result: Arc::new(Mutex::new(None)),
            last_format_toast: Mutex::new(None),
            pending_format_undo: Mutex::new(None),
            // MT-051 line-edit transforms: undo snapshot empty; tab settings default to VS Code's 4 spaces
            // (insert_spaces=true). The host overrides them from the operator's editor settings via
            // set_indent_settings; the dispatch reads them into a LineEditContext each batch (MC-006).
            pending_line_op_undo: Mutex::new(None),
            // MT-046 copy-as-note-reference + MT-070 create-note-from-link: nothing staged until the
            // context-menu entry / command dispatch fires.
            pending_copy_note_reference: Mutex::new(None),
            pending_create_note_link: Mutex::new(None),
            wikilink_resolver_index: Mutex::new(None),
            context_menu_open_for_snapshot: std::sync::atomic::AtomicBool::new(false),
            snapshot_capture_mode: std::sync::atomic::AtomicBool::new(false),
            // MT-071: seed indent from the document so the Indent segment + Tab key reflect the file's
            // actual style on open (tab-indented file -> Tabs; 4-space file -> Spaces 4), defaulting to
            // VS Code's Spaces 4 when ambiguous (MC-007). The host may still override via
            // set_indent_settings; the line-edit dispatch reads these into a LineEditContext each batch.
            tab_size: AtomicU64::new(detected_indent.size.max(1) as u64),
            insert_spaces: std::sync::atomic::AtomicBool::new(matches!(
                detected_indent.kind,
                super::file_meta::IndentKind::Spaces
            )),
            // MT-035 live text-edit undo: empty until a real typing/deletion/IME/newline path mutates.
            pending_text_edit_undo: Mutex::new(None),
            pending_code_edit_receipts: Mutex::new(VecDeque::new()),
            text_edit_undo_batcher: Mutex::new(CodeTextUndoBatcher::default()),
            // MT-071 file-metadata: seed EOL from the buffer (LF default — MC-007); language override
            // none (auto-detect); encoding UTF-8; render-whitespace off. These hang off the doc model so
            // they survive re-render + re-focus and the language resolver / draw path read them.
            language_override: Mutex::new(None),
            // MT-071 perf cache (must-fix #4): empty until the first resolve; recomputed only on a buffer
            // edit or an override change, so the per-frame status-bar resolve is a cheap key compare.
            resolved_language_cache: Mutex::new(None),
            eol: Mutex::new(detected_eol),
            encoding: Mutex::new(super::file_meta::Encoding::default()),
            render_whitespace: std::sync::atomic::AtomicBool::new(false),
            // MT-035: render-whitespace MODE (0=None default), plus the sticky-scroll + line-number visibility
            // toggles (both default ENABLED, matching the always-on pre-MT-035 behavior).
            render_whitespace_mode: std::sync::atomic::AtomicU8::new(0),
            sticky_scroll_enabled: std::sync::atomic::AtomicBool::new(true),
            // WP-KERNEL-012 wave-6 (S6 item 3): no live font-size / custom palette until the shell threads
            // them in from editor settings (None -> built-in MONO_FONT_SIZE + theme syntax tokens).
            font_size: Mutex::new(None),
            syntax_palette: Mutex::new(None),
            // MT-035 wave-7: no live line-height multiplier until the shell threads it in (None -> 1.0
            // single-spaced), and the matching-bracket + indent-guide chrome default ENABLED to match the
            // always-on pre-toggle behavior.
            line_height_multiplier: Mutex::new(None),
            bracket_matching_enabled: std::sync::atomic::AtomicBool::new(true),
            indent_guides_enabled: std::sync::atomic::AtomicBool::new(true),
            // MT-054 word wrap: OFF by default so the first render is the MT-002 1:1 baseline
            // (RISK-006 / MC-006). The viewport width is filled in each frame from the live editor-area
            // width before the wrap layout runs.
            wrap_config: Mutex::new(WrapConfig::default()),
            // MT-072 Fix 3: no pending USER wrap toggle until Alt+Z / the Wrap button / the
            // editor-wrap-toggle node flips it (a prefs->panel push never sets this).
            wrap_toggled_by_user: std::sync::atomic::AtomicBool::new(false),
            wrap_row_index: Mutex::new(None),
            live_text_node_id: Mutex::new(None),
            // MT-010 keymap: load any operator overrides from ~/.handshake/keymap.json (a missing file /
            // unresolvable home -> pure VS Code defaults), then merge them over the default table. The
            // override file path is resolved ONCE here (dirs::home_dir() — AC-007, no hardcoded path) so
            // the per-frame hot-reload poll does not re-resolve it.
            keymap: Mutex::new(Keymap::from_settings(&KeymapSettings::load_default())),
            keymap_version: AtomicU64::new(1),
            pending_chord: Mutex::new(None),
            keymap_file_path: keymap_settings_path().ok(),
            keymap_file_state: Mutex::new((None, None)),
            command_palette_tx: Mutex::new(None),
            command_node_cache: Mutex::new(None),
            editor_action_wiring: Mutex::new(None),
            // MT-034 code->notes: the NoteRefsPanel is hidden until the operator toggles it on (it loads
            // only on a symbol dwell). The dwell tracker + delivery cell start empty; the find-notes
            // backend defaults to the verified live search-v2 route (a test injects a mock).
            show_note_refs: std::sync::atomic::AtomicBool::new(false),
            note_refs_state: Mutex::new(NoteRefsState::Idle),
            note_refs_dwell: Mutex::new(SymbolDwellTracker::new()),
            note_refs_focused_symbol: Mutex::new(None),
            note_refs_result: Arc::new(Mutex::new(Vec::new())),
            note_refs_active_request: Mutex::new(None),
            note_refs_generation: AtomicU64::new(0),
            note_refs_observed_context: Mutex::new(None),
            find_notes_backend: Mutex::new(Arc::new(FindNotesHttp::production())),
            note_refs_dwell_threshold: Mutex::new(std::time::Duration::from_millis(
                crate::interop::NOTE_REFS_DWELL_MS,
            )),
            pending_note_ref_open: Mutex::new(None),
            // MT-076 IME: no composition in progress on a fresh panel (overlay-only; never in the buffer).
            preedit: Mutex::new(String::new()),
            live_find_node_id: Mutex::new(None),
        }
    }

    /// A cheap snapshot clone of the document buffer (ropey clones share structure O(1)). Returns an
    /// owned [`TextBuffer`] rather than a borrow because the buffer now lives behind a `Mutex` (MT-003:
    /// edits made it interior-mutable). Tests/later MTs read line counts / text through it.
    pub fn buffer(&self) -> TextBuffer {
        self.buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Run `f` against the locked buffer without cloning (the internal read path used by the render
    /// hot loop so it does not clone the rope every frame).
    fn with_buffer<R>(&self, f: impl FnOnce(&TextBuffer) -> R) -> R {
        let guard = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        f(&guard)
    }

    // ── MT-003 multi-cursor API (the deterministic surface AC-001..AC-006 + the input handler drive) ──

    /// A snapshot of the current cursor set (for tests / later MTs / the overlay). Cheap `Vec` clone.
    pub fn cursors(&self) -> CursorSet {
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// MT-031 (E5 melt-together): the PRIMARY selection as `(start, end, text)` BYTE range + its text,
    /// or `None` for a bare caret (no selected text). The text is sliced by BYTE RANGE from the rope
    /// (O(selection-length), never `.to_string()` on the whole document — the perf-lens cap / RISK-003),
    /// so the cross-pane selection-publish + Copy path stays cheap even on a multi-MB buffer. The range is
    /// clamped defensively so a stale range never panics (RISK-4 spirit).
    pub fn selected_primary_text(&self) -> Option<(usize, usize, String)> {
        let primary = self.cursors().primary();
        if !primary.is_selection() {
            return None;
        }
        let range = primary.range();
        let (start, end, text) = self.with_buffer(|b| {
            let len = b.len_bytes();
            let end = range.end.min(len);
            let start = range.start.min(end);
            (start, end, b.byte_slice_to_string(start..end))
        });
        if text.is_empty() {
            None
        } else {
            Some((start, end, text))
        }
    }

    /// Number of cursors currently active (>= 1 always).
    pub fn cursor_count(&self) -> usize {
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    /// Replace the whole cursor set with one caret at `byte_offset` (a plain, non-Alt click). Clamped
    /// + char-snapped to the buffer.
    pub fn set_single_cursor(&self, byte_offset: usize) {
        let before = self.primary_cursor_offset();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_primary(byte_offset, &buffer);
        drop(buffer);
        if self.primary_cursor_offset() != before {
            self.cancel_automatic_completion();
            self.close_completion();
            self.close_hover();
            self.reset_note_refs_context();
        }
    }

    /// Add a bare caret at `byte_offset` (Alt+Click / programmatic). De-duped + merged on insert.
    pub fn add_cursor_at(&self, byte_offset: usize) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add_cursor(byte_offset, &buffer);
    }

    /// Add a caret one line above / below every existing cursor (Ctrl+Alt+Up / Ctrl+Alt+Down).
    pub fn add_cursor_above(&self) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add_cursor_above(&buffer);
    }

    /// Add a caret one line below every existing cursor (Ctrl+Alt+Down).
    pub fn add_cursor_below(&self) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .add_cursor_below(&buffer);
    }

    /// Replace the cursor set with `cursors` (used by box/column selection — one cursor per line).
    pub fn set_cursors(&self, cursors: Vec<Cursor>) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_cursors(cursors, &buffer);
    }

    /// Ctrl+D, Monaco semantics:
    /// - If the primary cursor is a BARE CARET on a word, the FIRST Ctrl+D selects that word in place
    ///   (one selection) and returns. (The next press adds the next occurrence.)
    /// - If the primary is a SELECTION, add a selection over the NEXT occurrence of the same text,
    ///   skipping occurrences a cursor already covers.
    ///
    /// Wrap-around safe (RISK-003 / MC-003): the search wraps once; if every occurrence of the text is
    /// already selected, this is a NO-OP (returns `false`) rather than looping or adding a duplicate.
    /// Returns `true` only when a cursor was added or the bare-caret word selection happened.
    pub fn select_next_occurrence(&self) -> bool {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let mut set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
        let primary = set.primary();

        // Bare caret: first Ctrl+D just selects the word under the caret (and stops).
        if !primary.is_selection() {
            let word = word_at(primary.head, &buffer);
            if word.start == word.end {
                return false; // not on a word
            }
            set.set_cursors(vec![Cursor::selection(word.start, word.end)], &buffer);
            return true;
        }

        // Selection: find the next occurrence of the selected text, skipping ones already selected.
        let range = primary.range();
        let text = buffer.to_string();
        let needle = text.get(range.clone()).unwrap_or("").to_owned();
        if needle.is_empty() {
            return false;
        }
        // The set of ranges already covered by a cursor, so the wrap never re-selects an existing one.
        let existing: Vec<std::ops::Range<usize>> =
            set.cursors().iter().map(|c| c.range()).collect();

        // Walk forward from the primary's end, wrapping once, until we find an occurrence that is NOT
        // already selected. Bounded by the number of occurrences (each step advances `from`).
        let mut from = range.end;
        // The first candidate could be the wrap back to the very first occurrence; cap iterations at the
        // buffer length so a degenerate input cannot loop (each found advances `from` by >= 1).
        let mut guard = 0usize;
        let max_iter = text.len() + 2;
        while guard < max_iter {
            guard += 1;
            match find_next_occurrence(&needle, from, &buffer) {
                Some(found) => {
                    if existing.contains(&found) {
                        // Already selected. If this is the only/next occurrence and it equals the
                        // primary, every occurrence is covered -> stop (RISK-003 no-op).
                        if found == range {
                            return false;
                        }
                        // Advance past this already-selected occurrence and keep looking.
                        from = found.end.max(found.start + 1);
                        continue;
                    }
                    set.add_selection(found.start, found.end, &buffer);
                    return true;
                }
                None => return false,
            }
        }
        false
    }

    /// Insert `text` at every cursor (replacing selections), then re-highlight. Returns the number of
    /// insertions applied. The MT step-7 text-input entry point.
    pub fn insert_text(&self, text: &str) -> usize {
        // A trigger-character request remains anchored while the operator continues ordinary typing
        // at that exact caret during the debounce window (`.a`, `::na`, `_id`). Cursor movement,
        // selection replacement, deletion, file/workspace changes, and every non-insertion refresh
        // still cancel it through the existing invalidation paths.
        let automatic_continuation = self.completion_request.load(Ordering::Relaxed)
            == COMPLETION_REQUEST_AUTOMATIC
            && *self
                .automatic_completion_cursor
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                == Some(self.primary_cursor_offset())
            && {
                let (start, end) = self.primary_selection_bytes();
                start == end
            };
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            self.cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert_at_all(text, &mut buffer)
        };
        if applied > 0 {
            self.refresh(); // bump version + recompute highlights (RISK-002 invalidation).
            if automatic_continuation {
                self.completion_request
                    .store(COMPLETION_REQUEST_AUTOMATIC, Ordering::Relaxed);
                *self
                    .automatic_completion_cursor
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(self.primary_cursor_offset());
            }
        }
        applied
    }

    /// WP-KERNEL-012 MT-076 (E13 IME): handle one [`egui::ImeEvent`] for the code editor, mirroring the
    /// rich editor's `ime_handler`:
    /// - `Enabled` / `Preedit(s)` -> set the OVERLAY preedit text (NO buffer mutation — RISK-1 / MC-1).
    /// - `Commit(s)` -> CLEAR the preedit overlay, then INSERT `s` at every cursor via the proven
    ///   char-correct [`insert_text`](Self::insert_text) path (the ONE place code text is produced), so a
    ///   composed CJK string lands char-correct and the caret advances past it. An EMPTY commit (cancel)
    ///   just clears the overlay with no insert (AC3).
    /// - `Disabled` -> clear the overlay (composition cancelled), no buffer change.
    ///
    /// Returns `true` when the buffer was mutated (a non-empty Commit), so the caller marks the edit.
    /// The preedit is overlay-only: it is painted underlined at the primary caret by
    /// [`paint_cursor_overlay`](Self::paint_cursor_overlay) and is never in the buffer, so the
    /// double-insert bug that MT-012 fixed cannot recur here.
    pub fn handle_ime_event(&self, event: &egui::ImeEvent) -> bool {
        match event {
            egui::ImeEvent::Enabled => {
                self.set_preedit(String::new());
                false
            }
            egui::ImeEvent::Preedit(s) => {
                self.set_preedit(s.clone());
                false
            }
            egui::ImeEvent::Commit(s) => {
                // Clear the overlay BEFORE inserting so the preedit text and the commit can never both
                // land (the preedit is overlay-only, so this is just dropping the overlay). An empty
                // commit is the cancel path: clear, no insert.
                self.set_preedit(String::new());
                if s.is_empty() {
                    return false;
                }
                let applied = self.insert_text(s);
                applied > 0
            }
            egui::ImeEvent::Disabled => {
                self.set_preedit(String::new());
                false
            }
        }
    }

    /// MT-076: the current IME composition (preedit) text, or empty when no composition is active. Read
    /// by the render overlay and by tests asserting the cancel/commit clears it.
    pub fn preedit(&self) -> String {
        self.preedit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// MT-076: replace the IME preedit overlay text (internal; the input path + `handle_ime_event` drive
    /// it). Setting it triggers no buffer mutation and no highlight refresh — the preedit is a paint-only
    /// overlay, so it does not bump `buffer_version`.
    fn set_preedit(&self, text: String) {
        *self.preedit.lock().unwrap_or_else(|e| e.into_inner()) = text;
    }

    /// Replace the WHOLE buffer with a rope snapshot and re-highlight (MT-035 undo-snapshot restore).
    /// The unified undo scope's `undo_fn` for a code edit captures a [`TextBuffer`] snapshot taken BEFORE
    /// the edit (ropey clones are O(1) — implementation note 1/2) and calls this to restore it on Ctrl+Z.
    /// Bumping the buffer version through [`Self::refresh`] invalidates the stale highlight spans
    /// (RISK-002, the length-changing-undo case the buffer-version hook documents). Cursors are clamped to
    /// the new length so a restored shorter document never leaves an out-of-range caret (panic-free —
    /// AC-006 spirit). Returns the new byte length.
    pub fn set_buffer_snapshot(&self, snapshot: TextBuffer) -> usize {
        let new_len = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            *buffer = snapshot;
            let len = buffer.len_bytes();
            // Collapse to a single primary caret clamped into the restored buffer so a shrink does not
            // leave a stale out-of-range cursor (set_primary clamps the offset to the new length).
            let prior = self
                .cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .primary()
                .min();
            self.cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .set_primary(prior.min(len), &buffer);
            len
        };
        self.refresh();
        self.reset_note_refs_context();
        new_len
    }

    /// Replace the WHOLE buffer with `text` and re-highlight (MT-035 undo-snapshot restore). External
    /// callers keep the string API; undo/redo uses [`Self::set_buffer_snapshot`] to avoid stringifying
    /// large rope snapshots on the frame path.
    pub fn set_text(&self, text: &str) -> usize {
        self.set_buffer_snapshot(TextBuffer::new(text))
    }

    /// Delete at every cursor (selection, else the char before the caret — Backspace), then
    /// re-highlight. Returns the number of deletions applied.
    pub fn delete_text(&self) -> usize {
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            self.cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .delete_at_all(&mut buffer)
        };
        if applied > 0 {
            self.refresh();
        }
        applied
    }

    /// Build a box/column selection across `line_a..=line_b` (inclusive in either order) selecting
    /// `col_a..col_b` (inclusive of the smaller, exclusive of the larger column) on each line. One
    /// cursor per line, each clamped to that line's length (RISK-002). The Alt+Shift drag handler and
    /// the deterministic column-select test (AC-002) both call this.
    pub fn set_box_selection(&self, line_a: usize, col_a: usize, line_b: usize, col_b: usize) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let (l0, l1) = (line_a.min(line_b), line_a.max(line_b));
        let (c0, c1) = (col_a.min(col_b), col_a.max(col_b));
        let mut cursors = Vec::with_capacity(l1 - l0 + 1);
        for line in l0..=l1 {
            let anchor = line_col_to_byte(line, c0, &buffer);
            let head = line_col_to_byte(line, c1, &buffer);
            // A line shorter than c0 yields anchor == head (an empty caret on that line) — still a
            // valid box-selection row, matching Monaco (empty selection on short lines).
            cursors.push(Cursor::selection(anchor, head));
        }
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_cursors(cursors, &buffer);
    }

    // ── MT-004 find/replace API (the deterministic surface AC-001..AC-006 + the find bar UI drive) ──

    /// Open the find bar (Ctrl+F: `show_replace=false`; Ctrl+H: `show_replace=true`). If the primary
    /// cursor has a selection, the selected text pre-populates the query (Monaco/VS Code behavior —
    /// implementation note 4). Idempotent: re-opening keeps the existing query but updates
    /// `show_replace` (so Ctrl+H from an open find bar reveals the replace row). Runs an initial search
    /// so matches + the counter are populated immediately.
    pub fn open_find(&self, show_replace: bool) {
        self.open_find_with_focus(show_replace, true);
    }

    /// Open the find bar as the inactive half of the app-wide shared Find surface. It runs the same native
    /// engine but deliberately does not request keyboard focus, so the active pane remains the one actual
    /// query author and two mounted panes never fight for focus.
    pub fn open_find_passive(&self, show_replace: bool) {
        self.open_find_with_focus(show_replace, false);
    }

    fn open_find_with_focus(&self, show_replace: bool, request_focus: bool) {
        let selected = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            let primary = set.primary();
            if primary.is_selection() {
                let text = buffer.to_string();
                text.get(primary.range()).map(|s| s.to_owned())
            } else {
                None
            }
        };
        {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let state = guard.get_or_insert_with(FindState::default);
            state.show_replace = show_replace;
            if let Some(sel) = selected {
                if !sel.is_empty() && !sel.contains('\n') {
                    state.query.pattern = sel;
                }
            }
        }
        // MT-108 (MT-004 residual): auto-focus the find input on the next frame (VS Code parity), so the
        // operator can type immediately after Ctrl+F and a kittest can drive the real TextEdit.
        self.find_focus_pending
            .store(request_focus, Ordering::Release);
        self.refresh_find_matches();
    }

    /// Close the find bar: clears `find_state` so no match highlights paint on the next frame (AC-006).
    pub fn close_find(&self) {
        *self.find_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
        self.find_focus_pending.store(false, Ordering::Release);
        self.find_text_input_focused.store(false, Ordering::Release);
    }

    /// The find TextEdit owns keyboard input as soon as opening schedules focus, including the opening
    /// frame before egui can report `has_focus()`. This closes the command+text same-frame race where a
    /// character intended for the newly opened query field could otherwise also enter the code buffer.
    fn find_text_surface_owns_keyboard(&self) -> bool {
        self.find_focus_pending.load(Ordering::Acquire)
            || self.find_text_input_focused.load(Ordering::Acquire)
    }

    /// True when the find bar is open (a frame would paint match highlights). The render loop and tests
    /// read this; `find_state().is_some()` is the native analog of Monaco's `findWidgetVisible`.
    pub fn is_find_open(&self) -> bool {
        self.find_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// A snapshot clone of the current find state (for tests + the overlay). `None` when the bar is
    /// closed.
    pub fn find_state(&self) -> Option<FindState> {
        self.find_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Advance to the next match (wrapping at the end), and scroll the viewport to it. No-op when the
    /// bar is closed or there are no matches.
    pub fn next_match(&self) {
        self.step_match(true);
    }

    /// Go to the previous match (wrapping at the start), and scroll the viewport to it. No-op when the
    /// bar is closed or there are no matches.
    pub fn prev_match(&self) {
        self.step_match(false);
    }

    fn step_match(&self, forward: bool) {
        let target = {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_mut() else { return };
            if state.matches.is_empty() {
                return;
            }
            let n = state.matches.len();
            state.current_match = if forward {
                (state.current_match + 1) % n
            } else {
                (state.current_match + n - 1) % n
            };
            let show_replace = state.show_replace;
            state.current().map(|m| (m.line, show_replace))
        };
        if let Some((line, show_replace)) = target {
            // RISK-004: scroll so the match lands just BELOW the pinned find bar, not hidden behind it.
            self.scroll_to_match_line(line, show_replace);
        }
    }

    /// Scroll so `line` lands just BELOW the floating find bar instead of at the very top of the
    /// viewport, where the pinned find widget would occlude it (MT-108 residual for MT-004 / RISK-004).
    /// The inset is the find-bar height (taller in replace mode) plus its top margin and a reveal gap.
    /// Clamped to 0 so a match near the top of the document still scrolls to the top (it cannot scroll
    /// above it). Uses the cached measured line height; before the first measure the offset is 0 and the
    /// following frame re-derives it once the height is known (same one-shot contract as
    /// [`scroll_to_line`](Self::scroll_to_line)).
    fn scroll_to_match_line(&self, line: usize, show_replace: bool) {
        let lh = self
            .line_height_px
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(0.0);
        let bar_height = if show_replace {
            FIND_BAR_HEIGHT_REPLACE_PX
        } else {
            FIND_BAR_HEIGHT_SINGLE_PX
        };
        let inset = FIND_BAR_TOP_MARGIN_PX + bar_height + FIND_BAR_MATCH_REVEAL_GAP_PX;
        let offset = (line as f32 * lh - inset).max(0.0);
        self.scroll_to_offset_px(offset);
    }

    /// Set the query pattern (called by the find input each frame when the text changes) and re-search.
    /// A no-op when the bar is closed.
    pub fn set_find_query(&self, pattern: impl Into<String>) {
        {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_mut() else { return };
            let pattern = pattern.into();
            if state.query.pattern != pattern {
                state.query.pattern = pattern;
                // A new query clears the original-match continuation plan.
                state.replace_all_remaining = 0;
                state.replace_all_plan = None;
            }
        }
        self.refresh_find_matches();
    }

    /// Set a toggle (case-sensitive / whole-word / regex) and re-search. A no-op when the bar is closed.
    pub fn set_find_toggles(&self, case_sensitive: bool, whole_word: bool, is_regex: bool) {
        {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_mut() else { return };
            let toggles_changed = state.query.case_sensitive != case_sensitive
                || state.query.whole_word != whole_word
                || state.query.is_regex != is_regex;
            state.query.case_sensitive = case_sensitive;
            state.query.whole_word = whole_word;
            state.query.is_regex = is_regex;
            if toggles_changed {
                state.replace_all_remaining = 0;
                state.replace_all_plan = None;
            }
        }
        self.refresh_find_matches();
    }

    /// Set the replace text (called by the replace input). A no-op when the bar is closed.
    pub fn set_replace_text(&self, text: impl Into<String>) {
        let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = guard.as_mut() {
            let text = text.into();
            if state.replace_text != text {
                state.replace_text = text;
                state.replace_all_remaining = 0;
                state.replace_all_plan = None;
            }
        }
    }

    /// Replace the CURRENT match with the replace text, then re-search (RISK-003: the remaining match
    /// offsets are stale after a buffer edit, so we always re-run search before reusing the list).
    /// Returns `true` when a replacement was applied. The current-match index is preserved (clamped to
    /// the new, smaller match count) so repeated Replace walks through the occurrences.
    pub fn replace_current(&self) -> bool {
        let (target, replacement) = {
            let guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_ref() else {
                return false;
            };
            match state.current() {
                Some(m) => (m.clone(), state.replace_text.clone()),
                None => return false,
            }
        };
        let before = self.buffer();
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            FindEngine::replace_one(&mut buffer, &target, &replacement)
        };
        if applied {
            self.refresh(); // re-highlight (RISK-002 invalidation, edit changed the buffer)
            self.refresh_find_matches(); // RISK-003: recompute the now-stale match list
            if let Some(state) = self
                .find_state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_mut()
            {
                state.replace_all_remaining = 0;
                state.replace_all_plan = None;
            }
            self.record_code_edit_mutation(&before, &self.buffer());
        }
        applied
    }

    /// Replace the next bounded batch from one ORIGINAL match set, then re-search for current
    /// highlights. The private continuation plan is independent of that refreshed list, so replacement
    /// text that equals or contains the query cannot make the next click restart at the top.
    pub fn replace_all(&self) -> usize {
        let mut live_version = self.buffer_version.load(Ordering::Acquire);
        let matches_are_stale = self
            .find_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .is_some_and(|state| state.matches_buffer_version != live_version);
        if matches_are_stale {
            // An ordinary edit invalidates every cached byte range. Refresh outside the find-state
            // lock (refresh_find_matches takes buffer -> find_state) before constructing a fresh plan.
            self.refresh_find_matches();
            live_version = self.buffer_version.load(Ordering::Acquire);
            if let Some(state) = self
                .find_state
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_mut()
            {
                state.replace_all_plan = None;
                state.replace_all_remaining = 0;
            }
        }
        let (batch, replacement, batch_delta, total_matches, next_match) = {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_mut() else {
                return 0;
            };
            let plan_is_current = state.replace_all_plan.as_ref().is_some_and(|plan| {
                plan.query == state.query
                    && plan.replacement == state.replace_text
                    && plan.expected_buffer_version == live_version
            });
            if !plan_is_current {
                state.replace_all_plan = if state.matches.is_empty() {
                    None
                } else {
                    Some(ReplaceAllPlan {
                        query: state.query.clone(),
                        replacement: state.replace_text.clone(),
                        matches: state.matches.clone(),
                        next_match: 0,
                        cumulative_byte_delta: 0,
                        expected_buffer_version: live_version,
                    })
                };
            }
            let Some(plan) = state.replace_all_plan.as_ref() else {
                state.replace_all_remaining = 0;
                return 0;
            };
            let end = (plan.next_match + REPLACE_ALL_CAP).min(plan.matches.len());
            let mut shifted = Vec::with_capacity(end - plan.next_match);
            let mut delta = 0i64;
            for original in &plan.matches[plan.next_match..end] {
                let start = original.byte_range.start as i128 + plan.cumulative_byte_delta as i128;
                let finish = original.byte_range.end as i128 + plan.cumulative_byte_delta as i128;
                if start < 0 || finish < start || finish > usize::MAX as i128 {
                    state.replace_all_plan = None;
                    state.replace_all_remaining = 0;
                    return 0;
                }
                let mut adjusted = original.clone();
                adjusted.byte_range = start as usize..finish as usize;
                shifted.push(adjusted);
                delta += state.replace_text.len() as i64
                    - (original.byte_range.end - original.byte_range.start) as i64;
            }
            (
                shifted,
                plan.replacement.clone(),
                delta,
                plan.matches.len(),
                end,
            )
        };

        let before = self.buffer();
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            FindEngine::replace_all(&mut buffer, &batch, &replacement)
        };
        if applied == batch.len() && applied > 0 {
            self.refresh();
            self.refresh_find_matches();
            // Replace All is a discrete operator command. Reset only coalescer timing so this batch
            // receives one fresh unified-undo entry without dropping an undrained earlier snapshot.
            self.reset_text_edit_undo_batch_timing();
            self.record_text_edit_undo(before, self.buffer(), "Replace All");
        }
        {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = guard.as_mut() {
                if applied != batch.len() {
                    // A stale/invalid range is a deterministic invalidation, never an infinite retry.
                    state.replace_all_plan = None;
                    state.replace_all_remaining = 0;
                } else if next_match >= total_matches {
                    state.replace_all_plan = None;
                    state.replace_all_remaining = 0;
                } else if let Some(plan) = state.replace_all_plan.as_mut() {
                    plan.next_match = next_match;
                    plan.cumulative_byte_delta += batch_delta;
                    plan.expected_buffer_version = self.buffer_version.load(Ordering::Acquire);
                    state.replace_all_remaining = total_matches - next_match;
                }
            }
        }
        applied
    }

    /// Re-run [`FindEngine::search`] for the current query over the current buffer and store the result
    /// in `find_state`, clamping `current_match` into range and recording the regex compile error
    /// (AC-003). The single place matches are recomputed; called when the query/toggles change and
    /// after any replace (RISK-003). A no-op when the bar is closed.
    fn refresh_find_matches(&self) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let buffer_version = self.buffer_version.load(Ordering::Acquire);
        let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = guard.as_mut() else { return };
        state.matches = FindEngine::search(&state.query, &buffer);
        state.matches_buffer_version = buffer_version;
        state.error = FindEngine::compile_error(&state.query).unwrap_or_default();
        state.last_searched = state.query.pattern.clone();
        state.last_toggles = (
            state.query.case_sensitive,
            state.query.whole_word,
            state.query.is_regex,
        );
        if state.matches.is_empty() {
            state.current_match = 0;
        } else if state.current_match >= state.matches.len() {
            state.current_match = state.matches.len() - 1;
        }
    }

    /// The current highlight spans (read by tests + later MTs' minimap/outline). Returns the cached
    /// span set, recomputing it first if the buffer version moved since the last cache fill.
    pub fn spans(&self) -> Vec<HighlightSpan> {
        self.ensure_highlight_cache();
        self.highlight_cache
            .lock()
            .ok()
            .and_then(|c| c.as_ref().map(|(spans, _)| spans.spans.clone()))
            .unwrap_or_default()
    }

    /// Number of currently cached highlight ranges without cloning the span vector. This polls the
    /// document-wide delivery; callers proving the immutable foreground emission use
    /// [`initial_span_count`](Self::initial_span_count) instead.
    pub fn span_count(&self) -> usize {
        self.ensure_highlight_cache();
        self.highlight_cache
            .lock()
            .ok()
            .and_then(|cache| cache.as_ref().map(|(spans, _)| spans.spans.len()))
            .unwrap_or_default()
    }

    /// Number of ranges emitted by the foreground parse, before any document-wide worker delivery.
    /// This value is immutable, so a fast worker cannot race a performance proof into observing the
    /// completed count as if it were the first-emission count.
    pub fn initial_span_count(&self) -> usize {
        self.initial_span_count
    }

    fn initial_highlight_status_value(&self) -> InitialHighlightStatus {
        match self.initial_highlight_status.load(Ordering::Acquire) {
            INITIAL_HIGHLIGHT_COMPLETE => InitialHighlightStatus::Complete,
            INITIAL_HIGHLIGHT_FAILED => InitialHighlightStatus::Failed,
            _ => InitialHighlightStatus::Pending,
        }
    }

    /// Poll and return the initial projection state. Polling preserves the previous headless behavior:
    /// callers that await completion without mounting the panel still submit and ingest the job, while
    /// construction itself remains free of background contention until first emission has returned.
    pub fn initial_highlight_status(&self) -> InitialHighlightStatus {
        let _pending = self.poll_initial_highlight();
        self.initial_highlight_status_value()
    }

    pub fn initial_highlight_failure(&self) -> Option<InitialHighlightFailure> {
        *self
            .initial_highlight_failure
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Whether the initial large-file document-wide capture projection has replaced the first-window
    /// cache. Small documents are complete immediately.
    pub fn initial_highlight_complete(&self) -> bool {
        self.initial_highlight_status() == InitialHighlightStatus::Complete
    }

    /// Re-run highlighting over the current buffer (called after an edit). Bumps `buffer_version` so
    /// the highlight cache is invalidated, then recomputes — this is the path an edit/undo/redo in
    /// MT-003 will call. No-op highlighter -> empty spans. `&self` (interior-mutable) so it composes
    /// with the `Arc`-held render panel.
    pub fn refresh(&self) {
        self.cancel_automatic_completion();
        {
            // Serialize invalidation with result ingestion: an old generation can neither overwrite a
            // newly edited cache nor change its terminal status after this critical section.
            let _poll_guard = self
                .initial_highlight_poll
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(cancel) = self
                .initial_highlight_cancel
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take()
            {
                cancel.store(true, Ordering::Release);
            }
            self.initial_highlight_generation
                .fetch_add(1, Ordering::AcqRel);
            self.buffer_version.fetch_add(1, Ordering::Relaxed);
            self.initial_highlight_status
                .store(INITIAL_HIGHLIGHT_COMPLETE, Ordering::Release);
            *self
                .initial_highlight_job
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .initial_highlight_rx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .initial_highlight_source
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }
        self.invalidate_stale_code_intelligence_overlays();
        self.ensure_highlight_cache();
    }

    /// Reconstruct a bounded worker job from the retained immutable source and completed tree. This
    /// method performs no query work; retry scheduling therefore remains O(1) on the UI thread.
    fn schedule_initial_highlight_retry(&self, version: u64, generation: u64) -> bool {
        let next_attempt = self
            .initial_highlight_attempt
            .load(Ordering::Acquire)
            .saturating_add(1);
        if next_attempt > INITIAL_HIGHLIGHT_MAX_ATTEMPTS {
            return false;
        }
        let source = self
            .initial_highlight_source
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|(_, source_version, source_generation)| {
                *source_version == version && *source_generation == generation
            })
            .map(|(source, _, _)| Arc::clone(source));
        let Some(source) = source else {
            return false;
        };
        if !initial_highlight_source_is_worker_eligible(source.len()) {
            return false;
        }
        let tree = self
            .highlighter
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .and_then(|highlighter| highlighter.tree().cloned());
        let Some(tree) = tree else {
            return false;
        };
        if let Some(cancel) = self
            .initial_highlight_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            cancel.store(true, Ordering::Release);
        }
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let (result_tx, result_rx) = mpsc::channel();
        *self
            .initial_highlight_job
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(InitialHighlightJob {
            source,
            tree,
            extension: self.extension.clone(),
            version,
            generation,
            had_initial_spans: self.initial_span_count > 0,
            cancel: Arc::clone(&cancel),
            result_tx,
            #[cfg(test)]
            test_fault: None,
        });
        *self
            .initial_highlight_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(result_rx);
        *self
            .initial_highlight_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(cancel);
        self.initial_highlight_attempt
            .store(next_attempt, Ordering::Release);
        true
    }

    #[cfg(test)]
    fn inject_initial_highlight_fault(&self, fault: InitialHighlightTestFault) {
        *self
            .initial_highlight_test_fault
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(fault);
    }

    fn release_initial_highlight_resources(&self) {
        *self
            .initial_highlight_job
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .initial_highlight_source
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .initial_highlight_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        if let Some(cancel) = self
            .initial_highlight_cancel
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            cancel.store(true, Ordering::Release);
        }
    }

    /// Fold a completed large-file background capture projection into the version-1 cache. Returns
    /// `true` while the worker is still pending so the host can request another quiet repaint.
    fn poll_initial_highlight(&self) -> bool {
        if self.initial_highlight_status_value() != InitialHighlightStatus::Pending {
            return false;
        }
        let _poll_guard = self
            .initial_highlight_poll
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if self.initial_highlight_status_value() != InitialHighlightStatus::Pending {
            return false;
        }

        let enqueue_failure = {
            let mut pending_job = self
                .initial_highlight_job
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            #[cfg(test)]
            let injected_fault = self
                .initial_highlight_test_fault
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .take();
            #[cfg(test)]
            if injected_fault == Some(InitialHighlightTestFault::QueueFull) {
                *self
                    .initial_highlight_failure
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) =
                    Some(InitialHighlightFailure::QueueSaturated);
                return true;
            }
            match pending_job.take() {
                Some(job) => {
                    #[cfg(test)]
                    let mut job = job;
                    #[cfg(test)]
                    let injected_unavailable = matches!(
                        injected_fault,
                        Some(InitialHighlightTestFault::SpawnUnavailable)
                            | Some(InitialHighlightTestFault::Disconnect)
                    );
                    #[cfg(not(test))]
                    let injected_unavailable = false;
                    if injected_unavailable {
                        Some(InitialHighlightDelivery::Error {
                            version: job.version,
                            generation: job.generation,
                            failure: InitialHighlightFailure::WorkerUnavailable,
                        })
                    } else if let Some(sender) = initial_highlight_worker_sender() {
                        #[cfg(test)]
                        {
                            job.test_fault = injected_fault;
                        }
                        match sender.try_send(job) {
                            Ok(()) => None,
                            Err(mpsc::TrySendError::Full(job)) => {
                                *pending_job = Some(job);
                                *self
                                    .initial_highlight_failure
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner()) =
                                    Some(InitialHighlightFailure::QueueSaturated);
                                return true;
                            }
                            Err(mpsc::TrySendError::Disconnected(job)) => {
                                Some(InitialHighlightDelivery::Error {
                                    version: job.version,
                                    generation: job.generation,
                                    failure: InitialHighlightFailure::WorkerUnavailable,
                                })
                            }
                        }
                    } else {
                        Some(InitialHighlightDelivery::Error {
                            version: job.version,
                            generation: job.generation,
                            failure: InitialHighlightFailure::WorkerUnavailable,
                        })
                    }
                }
                None => None,
            }
        };

        let delivery = enqueue_failure.or_else(|| {
            let mut receiver = self
                .initial_highlight_rx
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let Some(rx) = receiver.as_ref() else {
                return Some(InitialHighlightDelivery::Error {
                    version: self.buffer_version.load(Ordering::Acquire),
                    generation: self.initial_highlight_generation.load(Ordering::Acquire),
                    failure: InitialHighlightFailure::WorkerUnavailable,
                });
            };
            match rx.try_recv() {
                Ok(delivery) => {
                    *receiver = None;
                    Some(delivery)
                }
                Err(mpsc::TryRecvError::Empty) => None,
                Err(mpsc::TryRecvError::Disconnected) => {
                    *receiver = None;
                    Some(InitialHighlightDelivery::Error {
                        version: self.buffer_version.load(Ordering::Acquire),
                        generation: self.initial_highlight_generation.load(Ordering::Acquire),
                        failure: InitialHighlightFailure::WorkerUnavailable,
                    })
                }
            }
        });
        let Some(delivery) = delivery else {
            return true;
        };

        let version = self.buffer_version.load(Ordering::Acquire);
        let generation = self.initial_highlight_generation.load(Ordering::Acquire);
        let window = match delivery {
            InitialHighlightDelivery::Success {
                version: delivered_version,
                generation: delivered_generation,
                window,
            } if delivered_version == version && delivered_generation == generation => Some(window),
            InitialHighlightDelivery::Error {
                version: delivered_version,
                generation: delivered_generation,
                failure,
            } if delivered_version == version && delivered_generation == generation => {
                *self
                    .initial_highlight_failure
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(failure);
                if failure != InitialHighlightFailure::Cancelled
                    && self.schedule_initial_highlight_retry(version, generation)
                {
                    return true;
                }
                None
            }
            _ => {
                *self
                    .initial_highlight_failure
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) =
                    Some(InitialHighlightFailure::StaleDelivery);
                if self.schedule_initial_highlight_retry(version, generation) {
                    return true;
                }
                None
            }
        };

        if let Some(window) = window {
            *self
                .highlight_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some((window, version));
            *self
                .minimap_row_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.initial_highlight_status
                .store(INITIAL_HIGHLIGHT_COMPLETE, Ordering::Release);
        } else {
            // The foreground window remains useful. A failed full projection must never erase it.
            self.initial_highlight_status
                .store(INITIAL_HIGHLIGHT_FAILED, Ordering::Release);
        }
        self.release_initial_highlight_resources();
        false
    }

    /// Recompute the highlight cache iff it is missing or stale (its stored version != the current
    /// `buffer_version`). Idempotent and cheap on a cache hit (just a version compare). This is the
    /// single place spans are parsed, so the render path is guaranteed not to re-parse on a hit
    /// (MT-002 step 3).
    fn ensure_highlight_cache(&self) {
        let _pending = self.poll_initial_highlight();
        let version = self.buffer_version.load(Ordering::Relaxed);
        {
            let cache = self
                .highlight_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if matches!(cache.as_ref(), Some((_, v)) if *v == version) {
                return; // cache hit: no re-parse this frame (MT-002 step 3).
            }
        }
        // Miss: parse once, under the highlighter lock, then store the spans at this version.
        let bytes = self.with_buffer(|b| b.to_bytes());
        let spans = match self
            .highlighter
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            Some(hl) => hl.highlight(&bytes),
            None => Vec::new(),
        };
        *self
            .highlight_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            Some((HighlightSpanWindow::from_spans(spans), version));
    }

    /// Recompute the fold regions iff the buffer version moved since they were last computed (MT-005
    /// impl note 3: do NOT re-walk the tree every frame). Reuses the highlighter's CURRENT parse tree
    /// (the same tree `ensure_highlight_cache` just parsed), so there is no second parse. The recomputed
    /// regions are merged into the existing [`FoldSet`] via [`FoldSet::set_regions`], which preserves
    /// the folded flag of any region whose start line survives the edit (a user's collapsed regions
    /// stay collapsed across edits). A no-op highlighter / no-language document leaves the fold set
    /// empty. Call AFTER [`ensure_highlight_cache`] so the tree reflects the current buffer.
    fn ensure_fold_regions(&self) {
        let version = self.buffer_version.load(Ordering::Relaxed);
        if self.fold_version.load(Ordering::Relaxed) == version {
            return; // fold regions already current for this buffer version (MT impl note 3).
        }
        // Recompute from the highlighter's current tree (no second parse).
        let regions = {
            let highlighter = self.highlighter.lock().unwrap_or_else(|e| e.into_inner());
            match highlighter.as_ref().and_then(|hl| hl.tree()) {
                Some(tree) => {
                    let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
                    FoldProvider::new().compute(tree, &buffer, self.language_id)
                }
                None => Vec::new(),
            }
        };
        self.fold_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_regions(regions);
        self.fold_version.store(version, Ordering::Relaxed);
    }

    // ── MT-005 code-folding API (the deterministic surface AC-001..AC-006 + the render/keymap drive) ──

    /// A snapshot clone of the current fold set (regions + folded flags). For tests / the gutter
    /// (MT-007) / later MTs. Recomputes the regions first if the buffer version moved.
    pub fn fold_set(&self) -> FoldSet {
        self.ensure_fold_regions();
        self.fold_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Toggle the fold whose region starts on buffer line `start_line`. Returns `true` when a region
    /// existed on that line (folded state flipped), `false` otherwise. The gutter fold-triangle click
    /// handler (MT-007) and the Ctrl+Shift+[ / Ctrl+Shift+] keymap call this; idempotent in pairs
    /// (AC-006: two toggles return to the original state).
    pub fn toggle_fold(&self, start_line: usize) -> bool {
        self.ensure_fold_regions();
        let changed = self
            .fold_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .toggle(start_line);
        if changed {
            self.fold_state_changed();
        }
        changed
    }

    /// Set the folded state for the exact region starting at `start_line`. This is the deterministic
    /// target used by AccessKit `Expand`/`Collapse` requests on `code_editor_fold_{start_line}`.
    pub fn set_fold_state(&self, start_line: usize, folded: bool) -> bool {
        self.ensure_fold_regions();
        let changed = self
            .fold_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_folded(start_line, folded);
        if changed {
            self.fold_state_changed();
        }
        changed
    }

    /// Fold the innermost region that contains buffer line `line` (Ctrl+Shift+[ at the cursor). Picks
    /// the region with the LARGEST start line that still covers `line` (the innermost enclosing fold).
    /// Returns `true` when a region was folded. A no-op (false) when `line` is in no foldable region.
    pub fn fold_at_line(&self, line: usize) -> bool {
        self.set_fold_at_line(line, true)
    }

    /// Unfold the innermost folded region that contains `line` (Ctrl+Shift+]). Returns `true` when a
    /// region was unfolded.
    pub fn unfold_at_line(&self, line: usize) -> bool {
        self.set_fold_at_line(line, false)
    }

    /// Set the folded state of the innermost region enclosing `line` to `folded`. Returns `true` when a
    /// matching region's state changed.
    fn set_fold_at_line(&self, line: usize, folded: bool) -> bool {
        self.ensure_fold_regions();
        let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
        // The innermost region containing `line` is the one with the largest start_line that still
        // covers it (regions are sorted enclosing-first, so the LAST match in iteration order is the
        // innermost).
        let target = set
            .regions
            .iter()
            .filter(|r| r.start_line <= line && line <= r.end_line)
            .map(|r| r.start_line)
            .max();
        match target {
            Some(start_line) => {
                let changed = set.set_folded(start_line, folded);
                drop(set);
                if changed {
                    self.fold_state_changed();
                }
                changed
            }
            None => false,
        }
    }

    /// Any user-visible fold-state change invalidates secondary layout caches that key off folded
    /// content. The FoldSet owns visible-line map invalidation; the panel owns wrap row indexing.
    fn fold_state_changed(&self) {
        *self
            .wrap_row_index
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Fold the region at the primary cursor's line (Ctrl+Shift+[). Convenience wrapper that resolves
    /// the cursor line then calls [`fold_at_line`](Self::fold_at_line).
    pub fn fold_at_cursor(&self) -> bool {
        let line = self.primary_cursor_line();
        self.fold_at_line(line)
    }

    /// Unfold the region at the primary cursor's line (Ctrl+Shift+]).
    pub fn unfold_at_cursor(&self) -> bool {
        let line = self.primary_cursor_line();
        self.unfold_at_line(line)
    }

    /// The buffer line the primary cursor's head sits on (for the fold/unfold keymap).
    fn primary_cursor_line(&self) -> usize {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let head = self
            .cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .primary()
            .head;
        byte_to_line_col(head, &buffer).0
    }

    // ── MT-006 outline (symbol tree) API ──────────────────────────────────────────────────────────

    /// Recompute the outline symbols iff the buffer version moved since they were last computed
    /// (MC-002: do NOT re-walk the tree every frame, and reuse the SAME tree the highlighter already
    /// parsed — no second parse). Call AFTER [`ensure_highlight_cache`](Self::ensure_highlight_cache)
    /// so the tree reflects the current buffer.
    fn ensure_outline(&self) {
        let version = self.buffer_version.load(Ordering::Relaxed);
        if self.outline_version.load(Ordering::Relaxed) == version {
            return; // outline already current for this buffer version (MC-002).
        }
        let items = {
            let highlighter = self.highlighter.lock().unwrap_or_else(|e| e.into_inner());
            match highlighter.as_ref().and_then(|hl| hl.tree()) {
                Some(tree) => {
                    let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
                    OutlineProvider::compute(tree, &buffer, self.language_id)
                }
                None => Vec::new(),
            }
        };
        if self.outline_default_pending.swap(false, Ordering::AcqRel) && !items.is_empty() {
            self.show_outline.store(true, Ordering::Release);
        }
        *self.outline_items.lock().unwrap_or_else(|e| e.into_inner()) = items;
        self.outline_version.store(version, Ordering::Relaxed);
    }

    /// A snapshot of the current outline symbols (recomputing first if the buffer version moved). For
    /// tests / the outline panel / later MTs (in-file symbol jump — MT-053).
    pub fn outline_items(&self) -> Vec<OutlineItem> {
        self.ensure_outline();
        self.outline_items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Whether the outline side panel is currently shown.
    pub fn is_outline_shown(&self) -> bool {
        self.show_outline.load(Ordering::Relaxed)
    }

    /// Whether the minimap side panel is currently shown.
    pub fn is_minimap_shown(&self) -> bool {
        self.show_minimap.load(Ordering::Relaxed)
    }

    /// Show / hide the outline side panel (RISK-001 / MC-001 — keep the center editor usable on small
    /// screens). The toggle button + a swarm agent both drive this.
    pub fn set_show_outline(&self, shown: bool) {
        self.show_outline.store(shown, Ordering::Relaxed);
    }

    /// Show / hide the minimap side panel (RISK-001 / MC-001).
    pub fn set_show_minimap(&self, shown: bool) {
        self.show_minimap.store(shown, Ordering::Relaxed);
    }

    /// Toggle the outline panel visibility; returns the new state.
    pub fn toggle_outline(&self) -> bool {
        let next = !self.is_outline_shown();
        self.set_show_outline(next);
        next
    }

    /// Toggle the minimap panel visibility; returns the new state.
    pub fn toggle_minimap(&self) -> bool {
        let next = !self.is_minimap_shown();
        self.set_show_minimap(next);
        next
    }

    /// Navigate (scroll + move the primary caret) to buffer `line`, routed through the fold-aware
    /// visible<->buffer mapping (MT-005) so the editor lands on the right ROW even when folds collapse
    /// lines above the target. This is the single navigation primitive the outline click, the go-to-line
    /// submit, and the minimap click all funnel through (MT positioning note). The line is clamped to
    /// the live buffer; the caret is moved to the start byte of that line.
    pub fn navigate_to_line(&self, line: usize) {
        let (clamped, byte) = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let last = buffer.len_lines().saturating_sub(1);
            let clamped = line.min(last);
            let byte = buffer.line_to_byte(clamped).unwrap_or(0);
            (clamped, byte)
        };
        // Move the primary caret to the target line's start (a single caret there, like VS Code's
        // go-to-line). Done before the scroll so the caret overlay paints on the scrolled-to row.
        self.set_single_cursor(byte);
        // Scroll so the target is visible. `scroll_to_line` works in VISIBLE-line space (the same units
        // `show_rows` strides by), so map the buffer line to its visible-line index through the fold set
        // first (MT-005 fold-aware mapping) — a folded region above the target shifts its visible row up.
        let visible_line = self.buffer_line_to_visible_line(clamped);
        self.scroll_to_line(visible_line);
    }

    /// Navigate to an exact byte offset, preserving the requested column while reusing the same
    /// fold-aware vertical scroll path as line navigation. Offsets beyond EOF are clamped and snapped
    /// by the canonical cursor setter.
    pub fn navigate_to_byte_offset(&self, byte_offset: usize) {
        let line = self.with_buffer(|buffer| {
            buffer
                .byte_to_line(byte_offset.min(buffer.len_bytes()))
                .unwrap_or_else(|| buffer.len_lines().saturating_sub(1))
        });
        self.navigate_to_line(line);
        self.set_single_cursor(byte_offset);
    }

    /// Map a BUFFER line to its VISIBLE (post-fold) line index using the current fold set (MT-005). A
    /// buffer line hidden inside a folded region maps to the visible row of the fold's start line (the
    /// nearest visible line at/above it), so navigation lands on the collapsed summary row rather than a
    /// hidden row. Linear over the (cheap) visible map; the fold set rebuilds the map lazily on a
    /// fold-state change.
    fn buffer_line_to_visible_line(&self, buffer_line: usize) -> usize {
        let total = self.with_buffer(|b| b.len_lines());
        let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
        let visible = set.rebuild_visible_map_for(total);
        // visible_line_to_buffer_line is monotonic non-decreasing; find the largest visible index whose
        // buffer line is <= buffer_line (the nearest visible row at/above the target).
        let mut result = 0usize;
        for v in 0..visible {
            if set.visible_line_to_buffer_line(v) <= buffer_line {
                result = v;
            } else {
                break;
            }
        }
        result
    }

    // ── MT-006 go-to-line palette API (Ctrl+G) ────────────────────────────────────────────────────

    /// Open the go-to-line palette (Ctrl+G). The input is pre-populated with the primary cursor's
    /// 1-based line (VS Code behavior). Idempotent: re-opening re-seeds from the current cursor line.
    pub fn open_goto_line(&self) {
        let cursor_line = self.primary_cursor_line();
        *self
            .goto_line_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(GotoLineState::for_cursor_line(cursor_line));
    }

    /// Close the go-to-line palette (Escape / after a successful jump / clicking away). A no-op when
    /// already closed.
    pub fn close_goto_line(&self) {
        *self
            .goto_line_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// True while the go-to-line palette is open.
    pub fn is_goto_line_open(&self) -> bool {
        self.goto_line_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// A snapshot of the go-to-line palette state, or `None` when closed.
    pub fn goto_line_state(&self) -> Option<GotoLineState> {
        self.goto_line_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Set the go-to-line input text (the modal's TextEdit pushes the edited value here each frame), and
    /// re-parse it against the live buffer so `parsed` reflects validity. No-op when the palette is
    /// closed.
    pub fn set_goto_line_input(&self, input: impl Into<String>) {
        let len_lines = self.with_buffer(|b| b.len_lines());
        let mut guard = self
            .goto_line_state
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(state) = guard.as_mut() {
            state.input = input.into();
            state.reparse(len_lines);
        }
    }

    /// Submit the go-to-line palette (Enter / the Go button): parse the input, and if it is a valid
    /// numeric line, navigate there (fold-aware) and close the palette. Returns `true` when a navigation
    /// happened. A non-numeric / empty input does NOT navigate and does NOT close (AC-002: no crash, no
    /// navigation) — the modal stays open so the user can correct the input.
    pub fn submit_goto_line(&self) -> bool {
        let len_lines = self.with_buffer(|b| b.len_lines());
        let target = {
            let mut guard = self
                .goto_line_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            match guard.as_mut() {
                Some(state) => state.reparse(len_lines),
                None => None,
            }
        };
        match target {
            Some(line) => {
                // MT-052 jump-history record site #1 (goto-line): record the PRE-jump caret location so
                // Navigate Back can return here, BEFORE the caret moves to the target line.
                self.record_jump_origin();
                self.navigate_to_line(line);
                self.close_goto_line();
                true
            }
            None => false, // invalid input: no navigation, palette stays open (AC-002).
        }
    }

    // ── MT-053 in-file Go to Symbol palette (Ctrl+Shift+O) ─────────────────────────────────────────

    /// Open the in-file symbol palette (Ctrl+Shift+O / the GO-menu 'Go to Symbol in File…' item). This is
    /// the SINGLE entry point both the keybinding dispatch and the menu wiring call (AC-005), so the two
    /// can never diverge. It sources the symbols by flattening the CURRENT MT-006 outline (the list the
    /// panel already computed from the highlighter's tree — NO re-parse, RISK-002 / AC-007), mapped
    /// against the live buffer for the byte ranges. Idempotent: re-opening re-seeds from the current
    /// outline. STRICTLY DISTINCT from `open_command_palette` / the MT-030 global quick-switcher.
    pub fn open_symbol_palette(&self) {
        // Make sure the outline is current (the same MC-002 reuse the outline panel relies on — recompute
        // only on a version change, never a fresh parse here).
        self.ensure_outline();
        let outline = self.outline_items();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .open(&outline, &buffer);
    }

    /// Close the in-file symbol palette (Escape / after a confirmed jump / clicking away). No-op when
    /// already closed.
    pub fn close_symbol_palette(&self) {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .close();
    }

    /// True while the in-file symbol palette is open.
    pub fn is_symbol_palette_open(&self) -> bool {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_open()
    }

    /// Set the symbol-palette query text and re-filter (the modal's TextEdit pushes its edited value here
    /// each frame — the same pattern the go-to-line palette uses). No-op when the palette is closed.
    pub fn set_symbol_palette_query(&self, query: impl Into<String>) {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_query(query);
    }

    /// The current filtered + ranked symbol-palette rows (read-only snapshot for the renderer / tests).
    pub fn symbol_palette_results(&self) -> Vec<super::symbol_palette::FileSymbol> {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .results()
            .to_vec()
    }

    /// The selected row index in the symbol-palette results.
    pub fn symbol_palette_selected(&self) -> usize {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .selected_index()
    }

    /// Move the symbol-palette selection down/up one row (arrow-key nav). No-op when closed/empty.
    pub fn symbol_palette_select_next(&self) {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .select_next();
    }

    /// Move the symbol-palette selection up one row.
    pub fn symbol_palette_select_prev(&self) {
        self.symbol_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .select_prev();
    }

    /// Confirm the symbol-palette selection (Enter / row click): emit the [`SymbolPaletteAction::JumpTo`]
    /// and APPLY it through the EXISTING fold-aware navigate + caret API (no new scroll mechanism). Records
    /// the pre-jump origin so Navigate Back returns to the call site (MT-052 jump-history site, the same
    /// the outline-click jump uses), then scrolls to the symbol line and selects its declaration range.
    /// Returns `true` when a jump happened (a non-empty result set), `false` otherwise. Closes the palette
    /// on a successful confirm.
    pub fn confirm_symbol_palette(&self) -> bool {
        let action = {
            let mut palette = self
                .symbol_palette
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            palette.confirm()
        };
        match action {
            Some(super::symbol_palette::SymbolPaletteAction::JumpTo { line, byte_range }) => {
                self.apply_symbol_jump(line, byte_range);
                true
            }
            None => false,
        }
    }

    /// Apply a symbol-palette / future-caller JumpTo: record the jump origin (cross-file Navigate Back),
    /// place a SELECTION over the symbol's declaration `byte_range` (clamped to the live buffer — a stale
    /// range never panics, RISK-004 / MC-004), and scroll the viewport to `line` through the SAME
    /// fold-aware visible<->buffer mapping `navigate_to_line` uses (no new scroll mechanism). VS Code's
    /// quick-outline reveals + selects the symbol's range, so this selects the declaration line rather
    /// than dropping a bare caret.
    fn apply_symbol_jump(&self, line: usize, byte_range: std::ops::Range<usize>) {
        // MT-052 jump-history record site (in-file symbol jump): record the pre-jump caret BEFORE moving.
        self.record_jump_origin();
        // Clamp the selection range to the live buffer (RISK-004 — a range computed against a since-edited
        // buffer must never index past it).
        let (clamped, sel_start, sel_end) = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let last = buffer.len_lines().saturating_sub(1);
            let clamped = line.min(last);
            let len = buffer.len_bytes();
            let start = byte_range.start.min(len);
            let end = byte_range.end.min(len).max(start);
            (clamped, start, end)
        };
        // Place a selection over the symbol's declaration range (or a bare caret if the range is empty).
        if sel_end > sel_start {
            self.set_cursors(vec![Cursor::selection(sel_start, sel_end)]);
        } else {
            self.set_single_cursor(sel_start);
        }
        // Scroll fold-aware: map the buffer line to its visible row (a fold above the target shifts it up).
        let visible_line = self.buffer_line_to_visible_line(clamped);
        self.scroll_to_line(visible_line);
    }

    // ── MT-052 GO-menu navigation: diagnostic traversal (F8/Shift+F8) + jump history (Alt+Left/Right) ─

    /// The primary caret's position as a [`BufferPosition`] (line, column). The bridge from the editor's
    /// byte-offset cursor to diagnostic-traversal space.
    fn primary_caret_position(&self) -> BufferPosition {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let primary = self
            .cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .primary();
        BufferPosition::from_cursor(primary, &buffer)
    }

    /// Record the CURRENT caret location as the PRE-jump origin in the jump-history stack (MT-052). Called
    /// at the four navigation-jump dispatch sites (goto-def / references / outline-symbol / goto-line)
    /// BEFORE the caret moves, so Navigate Back can return here — including across files (the entry
    /// carries this panel's `file_path`). Coalescing + forward-tail truncation + the 50-entry cap live in
    /// [`JumpHistory::record`]. NOTE (RISK-006 / MC-006): only these four jump sites call this — ordinary
    /// typing / arrow-key caret moves do NOT, so Alt+Left steps one JUMP at a time, not one char.
    pub fn record_jump_origin(&self) {
        let entry = self.current_jump_entry();
        self.jump_history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .record(entry);
    }

    /// A [`JumpEntry`] for the panel's current file + primary caret position.
    fn current_jump_entry(&self) -> JumpEntry {
        JumpEntry::new(self.file_path(), self.primary_caret_position())
    }

    /// Test hook: record an EXPLICIT jump origin (rather than the live caret), so a test can seed the
    /// jump history with a cross-file origin to exercise the graceful different-file Navigate Back path
    /// (MC-005) without a live multi-file host.
    #[doc(hidden)]
    pub fn record_jump_origin_for_test(&self, entry: JumpEntry) {
        self.jump_history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .record(entry);
    }

    /// Go to the NEXT diagnostic marker (F8). Reads the live MT-007 gutter marker store + the primary
    /// caret, asks [`next_diagnostic`] for the first marker strictly after the caret (wrapping at the
    /// end), and on `Some(pos)` RECORDS the pre-jump location (so Navigate Back returns here) then moves
    /// the caret to that line via the shared [`navigate_to_line`](Self::navigate_to_line) primitive. A
    /// graceful no-op (no record, no move) when there are no diagnostics — `next_diagnostic` returns
    /// `None`.
    fn go_to_next_diagnostic(&self) {
        let markers = self.diagnostic_markers();
        let cursor = self.primary_caret_position();
        if let Some(target) = next_diagnostic(&markers, cursor) {
            self.record_jump_origin();
            self.navigate_to_line(target.line);
        }
    }

    /// Go to the PREVIOUS diagnostic marker (Shift+F8). Symmetric to
    /// [`go_to_next_diagnostic`](Self::go_to_next_diagnostic) via [`prev_diagnostic`].
    fn go_to_prev_diagnostic(&self) {
        let markers = self.diagnostic_markers();
        let cursor = self.primary_caret_position();
        if let Some(target) = prev_diagnostic(&markers, cursor) {
            self.record_jump_origin();
            self.navigate_to_line(target.line);
        }
    }

    /// Navigate BACK (Alt+Left): pop the jump-history one step toward the past and restore that location.
    /// Reuses the cross-file restore path: when the restored entry's `file_path` differs from this
    /// panel's current file, the caret-move is deferred to the host (a follow-on host-mount MT opens the
    /// other file) — in MT-052 scope a different-file Back updates the file_path label + records the
    /// target but the actual document swap is the host's job; a SAME-file Back moves the caret here. A
    /// MISSING / different file is handled gracefully (no panic, no spurious caret jump in the wrong
    /// file). A no-op when there is nothing to go back to.
    fn navigate_back(&self) {
        let current = self.current_jump_entry();
        let target = {
            let mut hist = self.jump_history.lock().unwrap_or_else(|e| e.into_inner());
            hist.back(current)
        };
        if let Some(entry) = target {
            self.apply_jump_target(entry);
        }
    }

    /// Navigate FORWARD (Alt+Right): step the jump-history one entry toward the future and restore that
    /// location. A no-op when already at the tail.
    fn navigate_forward(&self) {
        let target = {
            let mut hist = self.jump_history.lock().unwrap_or_else(|e| e.into_inner());
            hist.forward()
        };
        if let Some(entry) = target {
            self.apply_jump_target(entry);
        }
    }

    /// Apply a restored jump target: when it is in THIS file, move the caret to its line (the live,
    /// testable path); when it names a DIFFERENT file, the document swap is the host-mount MT's
    /// responsibility, so MT-052 records the intent on the panel's pending cross-file target rather than
    /// moving the caret in the wrong file (RISK-005 — never jump the caret to a line in a file that is
    /// not loaded). A missing/empty path is a graceful no-op. The pending target is observable so the host
    /// + tests can confirm the cross-file intent was produced.
    fn apply_jump_target(&self, entry: JumpEntry) {
        let target_path = entry.file_path.to_string_lossy().to_string();
        let current_path = self.file_path();
        if target_path == current_path {
            // Same file: move the caret here (live + kittest-provable).
            self.navigate_to_line(entry.position.line);
            *self
                .pending_cross_file_jump
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .pending_cross_file_jump_origin
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        } else {
            // Different file: the host opens it (follow-on host-mount MT). Park the intent; do NOT move
            // the caret in the current (wrong) file. RISK-005: graceful, no panic, history cursor already
            // advanced by back()/forward().
            tracing::debug!(
                target = %target_path,
                current = %current_path,
                line = entry.position.line,
                "MT-052 navigate: cross-file jump target parked for the host to open"
            );
            *self
                .pending_cross_file_jump
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(entry);
            *self
                .pending_cross_file_jump_origin
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = self
                .host_render_pane_id
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
        }
    }

    /// Whether Navigate Back would do something (drives the GO-menu Back item's enabled state).
    pub fn can_navigate_back(&self) -> bool {
        self.jump_history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .can_back()
    }

    /// Whether Navigate Forward would do something (drives the GO-menu Forward item's enabled state).
    pub fn can_navigate_forward(&self) -> bool {
        self.jump_history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .can_forward()
    }

    /// The pending cross-file jump target produced by a Navigate Back/Forward into a DIFFERENT file, or
    /// `None`. The host-mount MT (E11) drains this to open the target document; observable so a test can
    /// prove the cross-file intent without a live multi-file host.
    pub fn pending_cross_file_jump(&self) -> Option<JumpEntry> {
        self.pending_cross_file_jump
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Drain the pending cross-file target exactly once for the mounted host document opener.
    pub fn take_pending_cross_file_jump(&self) -> Option<JumpEntry> {
        let jump = self
            .pending_cross_file_jump
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        *self
            .pending_cross_file_jump_origin
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        jump
    }

    pub fn take_pending_cross_file_jump_with_origin(&self) -> Option<(JumpEntry, Option<PaneId>)> {
        let jump = self
            .pending_cross_file_jump
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()?;
        let origin = self
            .pending_cross_file_jump_origin
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        Some((jump, origin))
    }

    pub fn set_host_render_pane_id(&self, pane_id: Option<PaneId>) {
        *self
            .host_render_pane_id
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = pane_id;
    }

    pub fn host_incarnation(&self) -> u64 {
        self.host_incarnation
    }

    /// Test/diagnostic hook: a snapshot clone of the jump-history stack (for the jump_history proof to
    /// observe the live panel-side wiring, not just the pure-module unit tests).
    pub fn jump_history_snapshot(&self) -> JumpHistory {
        self.jump_history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    // ── MT-007 gutter / diagnostics / breakpoints API ─────────────────────────────────────────────

    /// Replace the diagnostic markers the gutter draws (severity dots + left bars + hover messages).
    /// This is the slot MT-008's LSP client fills: it calls `push_diagnostics(markers)` whenever the
    /// backend `listProblemGroups` data changes. Defined here (this MT) so MT-008 calls it without a
    /// re-implementation.
    ///
    /// CRITICAL (KERNEL_BUILDER gate): this does NOT bump `buffer_version`. The contract's step 5 text
    /// mentions `self.buffer_version += 1` but also admits it is "not needed" — and bumping it would
    /// needlessly trigger the MT-002 highlight-cache invalidation + tree re-parse on EVERY diagnostics
    /// push (an LSP pushes diagnostics frequently). Diagnostics live in INDEPENDENT state, so a push
    /// only swaps this list — no re-highlight, no re-fold, no re-outline.
    pub fn push_diagnostics(&self, markers: Vec<GutterMarker>) {
        *self
            .diagnostic_markers
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = markers;
    }

    /// Add a diagnostic carrying a related Handshake note. The gutter marker remains the canonical
    /// diagnostic source; the related-note map adds the rendered/steerable navigation chip required by
    /// IC-09 without encoding navigation data into the human message string.
    pub fn push_diagnostic_note_reference(
        &self,
        line: usize,
        severity: DiagnosticSeverity,
        message: impl Into<String>,
        document_id: impl Into<String>,
    ) {
        self.diagnostic_markers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(GutterMarker::diagnostic(line, severity, message));
        self.diagnostic_note_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(line, document_id.into());
    }

    /// Stable AccessKit author id for a diagnostic's related-note chip.
    pub fn diagnostic_note_reference_author_id(&self, line: usize) -> String {
        let base = format!("{CODE_EDITOR_DIAGNOSTIC_NOTE_REF_AUTHOR_PREFIX}{line}");
        if self.instance.is_empty() {
            base
        } else {
            format!("{base}#{}", self.instance)
        }
    }

    /// A snapshot of the current diagnostic markers (for tests / the gutter / MT-008).
    pub fn diagnostic_markers(&self) -> Vec<GutterMarker> {
        self.diagnostic_markers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// A snapshot clone of the breakpoint set (for tests / the gutter / a future DAP client).
    pub fn breakpoint_set(&self) -> BreakpointSet {
        self.breakpoint_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// True when buffer `line` carries a breakpoint.
    pub fn is_breakpoint_set(&self, line: usize) -> bool {
        self.breakpoint_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .contains(line)
    }

    /// Toggle the breakpoint on buffer `line` (the gutter breakpoint click + a future keymap call this).
    /// Adds the breakpoint if absent, removes it if present (idempotent in pairs — AC-002), then
    /// publishes the matching [`BreakpointEvent`] onto the debug-adapter channel. Returns the resulting
    /// [`BreakpointAction`]. The publish is non-blocking and discards on a dropped receiver (RISK-003):
    /// `send(event).ok()` on the unbounded channel never blocks and a missing DAP client is benign.
    pub fn toggle_breakpoint(&self, line: usize) -> BreakpointAction {
        let action = self
            .breakpoint_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .toggle(line);
        let file_path = self
            .file_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        // RISK-003 / MC-003: non-blocking publish; `.ok()` discards the Err when the receiver is gone.
        self.breakpoint_sender
            .send(BreakpointEvent {
                file_path,
                line,
                action,
            })
            .ok();
        action
    }

    /// Take the receive half of the breakpoint channel so a future debug-adapter (DAP) client can
    /// consume published [`BreakpointEvent`]s. Returns the receiver the FIRST time it is called and
    /// `None` afterward (a channel has one consumer). Until a client subscribes, the receiver is parked
    /// inside the panel so publishes are queued rather than dropped; after the receiver is taken and
    /// later dropped, publishes become a benign no-op (RISK-003).
    pub fn subscribe_breakpoints(&self) -> Option<mpsc::Receiver<BreakpointEvent>> {
        self.breakpoint_receiver
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Set the path of the file this panel edits (carried on every published `BreakpointEvent`).
    pub fn set_file_path(&self, path: impl Into<String>) {
        let path = path.into();
        let changed = {
            let mut current = self.file_path.lock().unwrap_or_else(|e| e.into_inner());
            let changed = *current != path;
            *current = path;
            changed
        };
        if changed {
            // The detected language's extension layer reads `file_path`, so a same-panel file swap is
            // a cache-key change even when the buffer version and explicit override are unchanged.
            // Without this invalidation, `foo.rs` -> `foo.py` can keep reporting Rust until the first
            // text edit, which also prevents the host from rebinding/retiring the correct LSP client.
            *self
                .resolved_language_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .lsp_diagnostics_version
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.cancel_automatic_completion();
            self.close_completion();
            self.close_hover();
            self.close_references();
            self.reset_note_refs_context();
        }
    }

    /// The path of the file this panel edits (empty for an in-memory buffer).
    pub fn file_path(&self) -> String {
        self.file_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// MT-047: the `file://` URI of the document for LSP requests (`textDocument/signatureHelp`), or
    /// `None` for an in-memory buffer with no file path (the LSP path is then skipped and the editor
    /// falls back to the code-nav signature). Mirrors the URI the MT-008 `did_open` path uses.
    fn lsp_uri(&self) -> Option<String> {
        let path = self.file_path();
        if path.trim().is_empty() {
            return None;
        }
        // Build a file URL from the path (absolute or relative — `Url::from_file_path` requires an
        // absolute path, so fall back to a manual `file://` prefix for a relative path so a test with a
        // bare name still yields a URI the request can carry).
        match std::path::Path::new(&path).canonicalize() {
            Ok(abs) => lsp_types::Url::from_file_path(&abs)
                .ok()
                .map(|u| u.to_string())
                .or_else(|| Some(format!("file:///{}", path.trim_start_matches('/')))),
            Err(_) => Some(format!("file:///{}", path.trim_start_matches('/'))),
        }
    }

    /// MT-047: the LSP `Position` (0-based line + UTF-16 code-unit character) for `byte_offset` in the
    /// buffer. LSP positions count UTF-16 units rather than UTF-8 bytes or Unicode scalar values, so an
    /// astral character before the caret contributes two. Never panics (clamps/snaps to the buffer).
    fn lsp_position_at(&self, byte_offset: usize) -> lsp_types::Position {
        self.with_buffer(|buffer| {
            let offset = byte_offset.min(buffer.len_bytes());
            let line = buffer.byte_to_line(offset).unwrap_or(0);
            let line_start = buffer.line_to_byte(line).unwrap_or(0);
            let utf16_units = buffer
                .byte_slice_to_string(line_start..offset)
                .encode_utf16()
                .count();
            lsp_types::Position {
                line: line as u32,
                character: utf16_units as u32,
            }
        })
    }

    /// Reset the gutter's per-file state when a new file is loaded into this panel (RISK-004): clears
    /// stale diagnostic markers so a previous file's errors do not appear on the new file, and seeds the
    /// new `file_path` for breakpoint events. (Breakpoints are intentionally NOT cleared here — they are
    /// per-file and a real editor that swaps the panel's document would build a fresh panel; this method
    /// is the seam a same-panel file swap uses, and a swap caller that wants a clean breakpoint slate
    /// can call `clear_breakpoints`.) MT-008's open-file path calls this.
    pub fn load_file(&self, path: impl Into<String>) {
        self.set_file_path(path);
        self.saved_buffer_version.store(
            self.buffer_version.load(Ordering::Relaxed),
            Ordering::Relaxed,
        );
        // RISK-004: clear stale diagnostics from the previous file (no version bump — diagnostics are
        // independent state).
        self.push_diagnostics(Vec::new());
        self.diagnostic_note_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Version of the buffer that last matched the loaded/saved file on disk.
    pub fn saved_buffer_version(&self) -> u64 {
        self.saved_buffer_version.load(Ordering::Relaxed)
    }

    /// Advance the durable baseline after the host atomically saves this exact buffer version.
    pub fn mark_buffer_version_saved(&self, version: u64) {
        self.saved_buffer_version.store(version, Ordering::Relaxed);
    }

    /// Clear every breakpoint (a full-file reset surface for a same-panel document swap).
    pub fn clear_breakpoints(&self) {
        *self
            .breakpoint_set
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = BreakpointSet::new();
    }

    /// A snapshot of the gutter feature flags.
    pub fn gutter_config(&self) -> GutterConfig {
        *self.gutter_config.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Replace the gutter feature flags (a settings change / agent toggling a column).
    pub fn set_gutter_config(&self, config: GutterConfig) {
        *self.gutter_config.lock().unwrap_or_else(|e| e.into_inner()) = config;
    }

    /// The stable AccessKit author_id for this panel's gutter strip, with the instance suffix.
    pub fn gutter_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_GUTTER_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for the breakpoint toggle on buffer `line`, with the instance
    /// suffix (`code_editor_breakpoint_{line}`).
    pub fn breakpoint_author_id(&self, line: usize) -> String {
        if self.instance.is_empty() {
            format!("{CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX}{line}")
        } else {
            format!(
                "{CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX}{line}#{}",
                self.instance
            )
        }
    }

    /// The stable AccessKit author_id for the diagnostic marker on buffer `line`, with the instance
    /// suffix (`code_editor_diagnostic_{line}`).
    pub fn diagnostic_author_id(&self, line: usize) -> String {
        if self.instance.is_empty() {
            format!("{CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX}{line}")
        } else {
            format!(
                "{CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX}{line}#{}",
                self.instance
            )
        }
    }

    /// The screen rect the gutter strip occupied on the most recent frame, or `None` before the first
    /// render. The basis for the AC-003/AC-005 gutter layout + click tests.
    pub fn last_gutter_rect(&self) -> Option<egui::Rect> {
        *self
            .last_gutter_rect
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// The buffer line of each PAINTED gutter row on the most recent frame, in painted order. The
    /// deterministic basis for the AC-004 (all 10 lines painted) + AC-006 (a folded body line is no
    /// longer painted) gutter tests.
    pub fn gutter_rows_for_test(&self) -> Vec<usize> {
        self.last_gutter_rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The current `buffer_version` counter, for the AC-007 perf-gate test that asserts
    /// `push_diagnostics` does NOT bump it (no highlight-cache / tree re-parse on a diagnostics push).
    pub fn buffer_version_for_test(&self) -> u64 {
        self.buffer_version.load(Ordering::Relaxed)
    }

    /// Whether a completion request is currently armed (Ctrl+Space / trigger char) and not yet consumed
    /// by the per-frame pump. The MT-008 live-path test reads it to prove the pump CONSUMED the arm in
    /// the same frame (it does not linger to fire on a later, unrelated frame).
    pub fn completion_request_armed_for_test(&self) -> bool {
        self.completion_request.load(Ordering::Relaxed) != COMPLETION_REQUEST_NONE
    }

    /// Monotonic hover request generation observed by live dwell-path tests. It advances only when
    /// [`trigger_hover`](Self::trigger_hover) is actually reached.
    pub fn hover_request_generation_for_test(&self) -> u64 {
        self.hover_generation.load(Ordering::Relaxed)
    }

    /// Monotonic proof that the real Peek/Go-to-Definition request path was reached.
    pub fn definition_request_generation_for_test(&self) -> u64 {
        self.definition_generation.load(Ordering::Relaxed)
    }

    /// Queue raw code-nav symbols exactly like the off-thread completion/hover tasks do. Tests use this
    /// to prove the UI-thread drain handles multiple same-frame result batches without losing staleness
    /// markers before render consumes them.
    pub fn queue_code_nav_symbols_for_test(
        &self,
        prefix: impl Into<String>,
        symbols: Vec<CodeSymbolNavProjection>,
    ) {
        let workspace_id = self.workspace_id();
        self.code_nav_symbols_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .push((workspace_id, prefix.into(), symbols));
    }

    // ── MT-008 code intelligence API (completion / hover / code-nav / LSP) ─────────────────────────

    /// Bind the active workspace id used for backend code-nav lookups. Empty = no workspace (code-nav
    /// requests short-circuit to empty, the React `activeWorkspaceId() == null` behavior).
    pub fn set_workspace_id(&self, workspace_id: impl Into<String>) {
        let workspace_id = workspace_id.into();
        let changed = {
            let mut current = self.workspace_id.lock().unwrap_or_else(|e| e.into_inner());
            let changed = *current != workspace_id;
            *current = workspace_id;
            changed
        };
        if changed {
            self.cancel_automatic_completion();
            self.close_completion();
            self.close_hover();
            self.close_references();
            self.reset_note_refs_context();
        }
    }

    /// The active workspace id (empty when unbound).
    pub fn workspace_id(&self) -> String {
        self.workspace_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Replace the backend code-nav client. Hosts/tests use this to point the live completion/hover
    /// fallback at a known backend endpoint while the egui thread keeps using the same nonblocking spawn
    /// path as production.
    pub fn set_code_nav_client(&self, client: CodeNavClient) {
        *self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = client;
        // Reject any response already in flight from the previous client before it can repopulate the
        // freshly cleared cache. The next trigger obtains a newer generation from these same counters.
        self.completion_generation.fetch_add(1, Ordering::Relaxed);
        self.hover_generation.fetch_add(1, Ordering::Relaxed);
        self.definition_generation.fetch_add(1, Ordering::Relaxed);
        self.references_generation.fetch_add(1, Ordering::Relaxed);
        self.code_nav_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.close_completion();
        self.close_hover();
        self.close_references();
        // NoteRefs first resolves the dwelled symbol through this client. Fence the previous client's
        // in-flight lookup/search chain so a late delivery cannot commit after backend replacement.
        self.reset_note_refs_context();
    }

    /// Replace the LSP client (e.g. install a configured language server, or a mock LSP in a test). The
    /// default is [`LspClient::disabled`] (graceful empty results — AC-004).
    pub fn set_lsp_client(&self, client: Arc<LspClient>) {
        if let Some(uri) = self.lsp_uri() {
            client.seed_injected_document_sync(&uri);
        }
        *self
            .lsp_diagnostics_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(client.subscribe_diagnostics());
        *self
            .lsp_diagnostics_version
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self.lsp_client.lock().unwrap_or_else(|e| e.into_inner()) = client;
        self.completion_generation.fetch_add(1, Ordering::Relaxed);
        self.hover_generation.fetch_add(1, Ordering::Relaxed);
        self.definition_generation.fetch_add(1, Ordering::Relaxed);
        self.references_generation.fetch_add(1, Ordering::Relaxed);
        self.close_completion();
        self.close_hover();
        self.close_references();
        *self
            .last_definition_target
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.last_lsp_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// A clone of the current LSP client handle (for the diagnostics-drain wiring + tests).
    pub fn lsp_client(&self) -> Arc<LspClient> {
        Arc::clone(&self.lsp_client.lock().unwrap_or_else(|e| e.into_inner()))
    }

    /// Inject the app's tokio runtime handle so the LIVE render/input loop can drive the off-thread
    /// completion/hover triggers (the same per-component injection pattern `BackendClient` and
    /// `ProjectTree` use). The host calls this once after building the panel (e.g. from
    /// `HandshakeApp::set_runtime_handle`). Until it is set, the live code-intelligence loop is a
    /// graceful no-op (a runtime-less test harness renders without spawning backend tasks).
    pub fn set_runtime(&self, handle: tokio::runtime::Handle) {
        *self.runtime.lock().unwrap_or_else(|e| e.into_inner()) = Some(handle);
    }

    /// A clone of the injected runtime handle, or `None` when the host has not injected one (the live
    /// code-intelligence loop short-circuits to a no-op in that case).
    fn runtime_handle(&self) -> Option<tokio::runtime::Handle> {
        self.runtime
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The primary cursor's head byte offset (the live-loop hover-dwell / completion-prefix anchor).
    fn primary_cursor_offset(&self) -> usize {
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .primary()
            .head
    }

    /// Current primary caret byte offset for navigation diagnostics and host-level proof.
    pub fn primary_cursor_byte_offset(&self) -> usize {
        self.primary_cursor_offset()
    }

    /// The identifier word the primary caret currently sits in/just-after (the hover target + the
    /// completion prefix), or an empty string when the caret is not in a word. Reuses the MT-003
    /// [`word_at`] scanner against the live buffer.
    fn word_at_primary_cursor(&self) -> String {
        let offset = self.primary_cursor_offset();
        self.with_buffer(|b| {
            let range = word_at(offset, b);
            if range.is_empty() {
                String::new()
            } else {
                b.to_string()
                    .get(range)
                    .map(|s| s.to_owned())
                    .unwrap_or_default()
            }
        })
    }

    /// MT-008 LIVE code-intelligence per-frame pump. Called once per `show()` AFTER the rows + cursor
    /// input are processed, so the cursor offset is current. It is the production path that reaches the
    /// off-thread triggers the AC tests exercise directly:
    /// - drains any pending LSP `publishDiagnostics` onto the gutter (AC-008);
    /// - advances the hover-dwell clock for the live cursor offset and, once the dwell elapses, fires a
    ///   backend hover lookup for the word under the caret (impl note 3);
    /// - if a completion request was armed this frame (Ctrl+Space or a trigger character — set by
    ///   `process_cursor_input`), fires the debounced backend completion lookup for the caret word.
    ///
    /// Every step is a no-op without an injected runtime (the triggers need a `Handle` to `spawn`), and
    /// the triggers themselves no-op when no workspace is bound — so a runtime-less / workspace-less
    /// harness still renders cleanly while a live host with a workspace gets real intelligence.
    fn pump_code_intelligence(&self) {
        // AC-008: route any LSP diagnostics notification to the gutter. Cheap when the channel is empty,
        // and independent of the runtime handle (the receiver is already on the panel).
        self.drain_lsp_diagnostics();
        self.invalidate_stale_code_intelligence_overlays();

        if self.completion_request.load(Ordering::Relaxed) == COMPLETION_REQUEST_AUTOMATIC {
            let anchored_cursor = *self
                .automatic_completion_cursor
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if anchored_cursor != Some(self.primary_cursor_offset()) {
                self.cancel_automatic_completion();
            }
        }

        let Some(runtime) = self.runtime_handle() else {
            // No runtime injected: clear any armed completion / signature-help request so it does not
            // fire later, and skip the off-thread triggers (the synthetic open_completion/open_hover/
            // open_signature_help test paths and the diagnostics drain above still work without a
            // runtime).
            self.completion_request
                .store(COMPLETION_REQUEST_NONE, Ordering::Relaxed);
            *self
                .automatic_completion_cursor
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            self.signature_help_request.store(false, Ordering::Relaxed);
            return;
        };

        // HOVER: advance the dwell clock for the live caret offset; on a dwell hit, fetch the hover for
        // the word under the caret (a no-op when the caret is not in a word / no workspace is bound).
        let offset = self.primary_cursor_offset();
        if self.update_hover_dwell(offset) {
            let word = self.word_at_primary_cursor();
            if !word.is_empty() {
                self.trigger_hover(&runtime, &word);
            }
        }

        // COMPLETION: an explicit Ctrl+Space bypasses debounce; an automatic trigger character remains
        // armed until the debounce expires instead of being consumed and lost on the first frame.
        let request_mode = self.completion_request.load(Ordering::Relaxed);
        let ready = request_mode == COMPLETION_REQUEST_EXPLICIT
            || (request_mode == COMPLETION_REQUEST_AUTOMATIC && self.completion_debounce_elapsed());
        if ready {
            self.completion_request
                .store(COMPLETION_REQUEST_NONE, Ordering::Relaxed);
            *self
                .automatic_completion_cursor
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            let word = self.word_at_primary_cursor();
            self.trigger_completion(&runtime, &word);
        }

        // MT-047 SIGNATURE HELP: fire only when armed this frame (a `(`/`,`/`)` trigger char or
        // Ctrl+Shift+Space). `trigger_signature_help` re-evaluates the enclosing call: it opens/updates
        // the popup when the cursor is inside a call and dismisses it when the cursor has left every
        // call (so a typed `)` that closes the call dismisses, while a `)` closing only a nested call
        // re-opens for the outer call).
        let sig_armed = self.signature_help_request.swap(false, Ordering::Relaxed);
        if sig_armed {
            self.trigger_signature_help(&runtime);
        }

        // MT-047 (AC-002) PER-FRAME DISMISSAL GUARD: once the popup is OPEN, close it when the caret has
        // left the anchored call OR the code text surface lost focus. These are the dismissal paths a
        // NON-TYPING caret move (ArrowLeft/Right/Up/Down, Home/End, or a mouse click) or a focus change
        // takes — none of them arm a trigger character, so without this guard the popup lingers at its
        // stale anchor and follows the cursor. The typing triggers ('(' / ',' / ')'), Escape, and the
        // in-flight-result drain do NOT cover a bare caret move. Reuses the EXISTING enclosing-call
        // scanner (`active_call_open_paren` -> `find_enclosing_open_paren`) rather than a second scan: the
        // caret is "still inside the call" ONLY when its enclosing open-paren is exactly the popup anchor,
        // so typing ',' between args (caret still inside the same call) keeps it open + updates the active
        // param. Runs INSIDE the runtime-present branch so the runtime-less synthetic-state render/AccessKit
        // proofs (which open the popup directly at a non-call anchor) are never spuriously dismissed.
        if let Some(state) = self.signature_help_state() {
            let cursor_byte = self.primary_cursor_offset();
            let caret_left_call =
                self.active_call_open_paren(cursor_byte) != Some(state.anchor_byte);
            let surface_lost_focus = !self.code_surface_focused.load(Ordering::Relaxed);
            if caret_left_call || surface_lost_focus {
                self.close_signature_help();
            }
        }
    }

    /// A snapshot of the completion popup state (`None` when no popup is showing). For tests + the
    /// input handler.
    pub fn completion_state(&self) -> Option<CompletionState> {
        self.completion_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// True while the completion popup is showing.
    pub fn is_completion_open(&self) -> bool {
        self.completion_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    fn completion_observer_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_COMPLETION_OBSERVER_AUTHOR_ID)
    }

    fn reset_completion_observer_for_popup(&self) {
        let context =
            completion_observer_context(&self.instance, &self.workspace_id(), &self.file_path());
        let mut observer = self
            .completion_observer
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let generation = observer
            .generation
            .checked_add(1)
            .expect("code completion observer generation exhausted");
        *observer = CompletionObserverState::ready(context, generation);
    }

    fn completion_observer_snapshot(&self) -> CompletionObserverState {
        self.completion_observer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    fn mark_completion_click_applied(
        &self,
        expected_context: &str,
        expected_generation: u64,
        completion_index: usize,
        semantic_value: &str,
    ) -> bool {
        let pending_target = completion_item_author_id(completion_index, &self.instance);
        let mut observer = self
            .completion_observer
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if observer.state != ClickCompletionState::Ready
            || observer.context != expected_context
            || observer.generation != expected_generation
        {
            return false;
        }
        let applied_generation = observer
            .generation
            .checked_add(1)
            .expect("code completion observer generation exhausted");
        if serialize_observer_click_state(
            CODE_EDITOR_COMPLETION_ACCEPT_EFFECT,
            &observer.context,
            applied_generation,
            ClickCompletionState::Applied,
            Some(&pending_target),
            Some(semantic_value),
        )
        .is_none()
        {
            return false;
        }
        observer.generation = applied_generation;
        observer.state = ClickCompletionState::Applied;
        observer.pending_target = Some(pending_target);
        observer.semantic_value = Some(semantic_value.to_owned());
        true
    }

    fn emit_completion_observer(&self, ctx: &egui::Context) {
        let observer = self.completion_observer_snapshot();
        let value = serialize_observer_click_state(
            CODE_EDITOR_COMPLETION_ACCEPT_EFFECT,
            &observer.context,
            observer.generation,
            observer.state,
            observer.pending_target.as_deref(),
            observer.semantic_value.as_deref(),
        )
        .expect("code completion observer fields are bounded and valid");
        let author_id = self.completion_observer_author_id();
        let node_id = egui::Id::new(("code-editor-completion-observer", &self.instance));
        ctx.accesskit_node_builder(node_id, move |node| {
            node.set_role(accesskit::Role::Status);
            node.set_author_id(author_id.clone());
            node.set_label("Code completion acceptance status".to_owned());
            node.set_value(value.clone());
        });
    }

    /// Open the completion popup with `items` anchored at the primary cursor's pixel (the deterministic
    /// path the trigger + tests use). A no-op when `items` is empty (nothing to show).
    pub fn open_completion(&self, items: Vec<CompletionItem>) {
        if items.is_empty() {
            self.close_completion();
            return;
        }
        let anchor = self
            .cursor_screen_pos()
            .unwrap_or_else(|| egui::pos2(40.0, 40.0));
        self.reset_completion_observer_for_popup();
        *self
            .completion_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(CompletionState::new(items, anchor));
        *self
            .completion_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(self.current_code_intelligence_identity(
            self.completion_generation.load(Ordering::Relaxed),
            String::new(),
        ));
    }

    /// Close the completion popup (Escape / after accept / no items).
    pub fn close_completion(&self) {
        self.completion_generation.fetch_add(1, Ordering::Relaxed);
        *self
            .completion_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .completion_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Move the completion selection down (ArrowDown). A no-op when closed.
    pub fn completion_select_next(&self) {
        if let Some(state) = self
            .completion_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            state.select_next();
        }
    }

    /// Move the completion selection up (ArrowUp). A no-op when closed.
    pub fn completion_select_prev(&self) {
        if let Some(state) = self
            .completion_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            state.select_prev();
        }
    }

    /// Accept the currently-selected completion item (Enter): insert its `insert_text` at the cursor,
    /// then close the popup. Returns `true` when an item was inserted. The single accept path the Enter
    /// keymap + the popup click both funnel through.
    pub fn accept_completion(&self) -> bool {
        let insert = {
            let guard = self
                .completion_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard
                .as_ref()
                .and_then(|s| s.selected().map(|i| i.insert_text.clone()))
        };
        match insert {
            Some(text) => {
                self.apply_text_edit_undoable("code: accept completion", |panel| {
                    panel.insert_text(&text)
                });
                self.close_completion();
                true
            }
            None => false,
        }
    }

    /// Accept the completion item at `index` (a click on a specific row). Inserts + closes.
    pub fn accept_completion_index(&self, index: usize) -> bool {
        let insert = {
            let guard = self
                .completion_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            guard
                .as_ref()
                .and_then(|s| s.items.get(index).map(|i| i.insert_text.clone()))
        };
        match insert {
            Some(text) => {
                self.apply_text_edit_undoable("code: accept completion", |panel| {
                    panel.insert_text(&text)
                });
                self.close_completion();
                true
            }
            None => false,
        }
    }

    /// A snapshot of the hover tooltip state (`None` when no tooltip is showing).
    pub fn hover_state(&self) -> Option<HoverState> {
        self.hover_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// True while the hover tooltip is showing.
    pub fn is_hover_open(&self) -> bool {
        self.hover_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// Open the hover tooltip with `state` (the deterministic path the dwell trigger + tests use).
    pub fn open_hover(&self, state: HoverState) {
        *self.hover_state.lock().unwrap_or_else(|e| e.into_inner()) = Some(state);
        *self
            .hover_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(self.current_code_intelligence_identity(
            self.hover_generation.load(Ordering::Relaxed),
            String::new(),
        ));
    }

    /// Close the hover tooltip (cursor moved / Escape / after go-to-def).
    pub fn close_hover(&self) {
        self.hover_generation.fetch_add(1, Ordering::Relaxed);
        *self.hover_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .hover_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    // ── MT-047 signature help (parameter hints) — public API + triggers ───────────────────────────

    /// A snapshot of the signature-help popup state (`None` when no popup is showing). The deterministic
    /// observation point for the kittest/unit proofs.
    pub fn signature_help_state(&self) -> Option<SignatureHelpState> {
        self.signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// True while the signature-help popup is showing.
    pub fn is_signature_help_open(&self) -> bool {
        self.signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// Open the signature-help popup with `state` (the deterministic path the trigger spawn delivers
    /// into + the tests drive directly).
    pub fn open_signature_help(&self, state: SignatureHelpState) {
        *self
            .signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(state);
    }

    /// Close the signature-help popup (`)`/Escape/cursor leaving the call/focus loss) and clear the
    /// fallback cache so a fresh call site re-resolves (RISK-002).
    pub fn close_signature_help(&self) {
        *self
            .signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .signature_fallback_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Mirror whether the code TEXT surface currently holds focus (AC-002 dismissal — scope step 8). The
    /// pane factory calls this every frame from its `has_focus` (`ui.memory().focused()` == the pane's egui
    /// id) BEFORE `show()`, so the per-frame signature-help dismissal guard can close the popup when the
    /// editor loses focus. Interaction tests drive it directly to prove the focus-loss dismissal path.
    pub fn set_code_surface_focus(&self, focused: bool) {
        self.code_surface_focused.store(focused, Ordering::Relaxed);
    }

    /// The number of overloads in the open signature-help popup (0 when closed). The input handler uses it
    /// to decide whether the popup OWNS Up/Down (overload cycling) so those keys are consumed instead of
    /// ALSO moving the caret (the peeked-not-consumed double-fire fix, applied only when >1 overload).
    fn signature_help_overload_count(&self) -> usize {
        self.signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|s| s.signatures.len())
            .unwrap_or(0)
    }

    /// Cycle the active overload to the NEXT signature (Down arrow while the popup is open). No-op when
    /// the popup is closed / there is only one signature.
    pub fn signature_help_next(&self) {
        if let Some(state) = self
            .signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            state.select_next_signature();
        }
    }

    /// Cycle the active overload to the PREVIOUS signature (Up arrow while the popup is open).
    pub fn signature_help_prev(&self) {
        if let Some(state) = self
            .signature_help_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            state.select_prev_signature();
        }
    }

    /// Whether a signature-help request is armed this frame (a trigger char / Ctrl+Shift+Space) and not
    /// yet consumed by the pump. Test-observable so a kittest can prove the live keystroke path arms it.
    pub fn signature_help_request_armed_for_test(&self) -> bool {
        self.signature_help_request.load(Ordering::Relaxed)
    }

    /// Find the byte offset of the open-paren of the call whose argument list the cursor is currently
    /// inside, or `None` when the cursor is not inside an unclosed `(` at the time of the call. Scans
    /// LEFT from the cursor over the buffer prefix, balancing `)` against `(` so a nested closed call is
    /// skipped; the first unbalanced `(` is the active call's open-paren. String/char literals are
    /// respected so a `(` inside a string is not treated as a call. This anchors the popup to a call
    /// site (the `anchor_byte`) and is the basis for dismissal (the cursor leaving the call).
    pub fn active_call_open_paren(&self, cursor_byte: usize) -> Option<usize> {
        let prefix =
            self.with_buffer(|b| b.byte_slice_to_string(0..cursor_byte.min(b.len_bytes())));
        find_enclosing_open_paren(&prefix)
    }

    /// The identifier token immediately to the LEFT of `open_paren_byte` (the call target), or an empty
    /// string when there is none. Used to resolve the fallback signature via the code-nav client.
    fn call_target_identifier(&self, open_paren_byte: usize) -> String {
        let prefix =
            self.with_buffer(|b| b.byte_slice_to_string(0..open_paren_byte.min(b.len_bytes())));
        identifier_before(&prefix)
    }

    /// Spawn the off-thread signature-help request: try the LSP server first (when one supports it),
    /// then fall back to the Handshake backend code-nav symbol under the call target. Delivers the
    /// resolved [`SignatureHelpState`] into `signature_help_result` (drained next frame). A no-op when
    /// the cursor is not inside a call (`active_call_open_paren` is `None`). The egui thread never blocks
    /// (HBR-QUIET): both the LSP request and the backend lookup run on the injected runtime.
    ///
    /// `anchor_byte` (the call's open-paren) keys the popup so a comma UPDATES the open popup rather than
    /// opening a second one (RISK-002). The fallback signature is cached per `(identifier, anchor_byte)`
    /// so commas in the same call do NOT re-hit `/knowledge/code/symbols` (RISK-002 / MC-002).
    pub fn trigger_signature_help(&self, runtime: &tokio::runtime::Handle) {
        let cursor_byte = self.primary_cursor_offset();
        let Some(open_paren_byte) = self.active_call_open_paren(cursor_byte) else {
            // Cursor is not inside a call: nothing to show; close any stale popup.
            self.close_signature_help();
            return;
        };
        // Compute the fallback active parameter locally from the top-level comma count between the
        // open-paren and the cursor (RISK-001 / AC-007). The LSP path overrides this with the server's
        // active_parameter when it answers.
        let slice = self.with_buffer(|b| {
            b.byte_slice_to_string(open_paren_byte..cursor_byte.min(b.len_bytes()))
        });
        let active_parameter = active_parameter_from_commas(&slice, open_paren_byte, cursor_byte);
        let identifier = self.call_target_identifier(open_paren_byte);

        let uri = self.lsp_uri();
        let position = self.lsp_position_at(cursor_byte);
        let lsp_client = self.lsp_client();
        let code_nav = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let workspace_id = self.workspace_id();
        let cell = Arc::clone(&self.signature_help_result);
        let fallback_cache = Arc::clone(&self.signature_fallback_cache);
        // The cached fallback symbol (if any) for THIS exact call site, so a comma re-trigger reuses it
        // instead of re-hitting the backend (RISK-002 / MC-002).
        let cached_fallback = {
            let guard = fallback_cache.lock().unwrap_or_else(|e| e.into_inner());
            match guard.as_ref() {
                Some((id, paren, sym)) if *id == identifier && *paren == open_paren_byte => {
                    Some(sym.clone())
                }
                _ => None,
            }
        };

        runtime.spawn(async move {
            // 1) LSP path: only issues a request when the server declared signatureHelpProvider (the
            // client's `signature_help` short-circuits to None otherwise). A present response wins.
            if let Some(uri) = uri.as_deref() {
                if let Some(help) = lsp_client.signature_help(uri, position).await {
                    if let Some(state) = SignatureHelpState::from_lsp(&help, open_paren_byte) {
                        if let Ok(mut slot) = cell.lock() {
                            *slot = Some(state);
                        }
                        return;
                    }
                }
            }
            // 2) Code-nav fallback: reuse the cached symbol for this call site, else resolve it once and
            // cache it against (identifier, open_paren_byte) so later commas reuse it (RISK-002).
            let symbol = if let Some(sym) = cached_fallback {
                Some(sym)
            } else if !workspace_id.is_empty() && !identifier.is_empty() {
                let matches = code_nav
                    .lookup_symbols(&workspace_id, &identifier, 5)
                    .await
                    .unwrap_or_default();
                // `lookup_symbols` is a PREFIX query, so `add` also matches `address`/`add_one`. Prefer
                // an EXACT `display_name == identifier` match (the backend `display_name` is the bare
                // call-target name) so the popup names the symbol actually being called; only if no
                // exact match exists do we fall back to the first prefix match (better than nothing).
                // This is the value-bearing half of the contract's `get_symbol` resolve step: the
                // per-entity `get_symbol` round-trip is skipped because it returns the SAME bare
                // `display_name` the lookup already carries (no richer parameter data exists — the
                // code-nav parameter-signature gap is the named NEEDS_MANAGED_RESOURCE_PROOF resource),
                // so it would add a backend round-trip without adding any signature content.
                let best = matches
                    .iter()
                    .find(|m| m.display_name == identifier)
                    .cloned()
                    .or_else(|| matches.into_iter().next());
                if let Some(sym) = &best {
                    if let Ok(mut slot) = fallback_cache.lock() {
                        *slot = Some((identifier.clone(), open_paren_byte, sym.clone()));
                    }
                }
                best
            } else {
                None
            };
            let Some(symbol) = symbol else {
                return; // no LSP, no symbol -> nothing renders (graceful, no panic — AC-003/AC-008).
            };
            if let Some(state) =
                SignatureHelpState::from_code_nav(&symbol, open_paren_byte, active_parameter)
            {
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(state);
                }
            }
        });
    }

    /// The screen pixel of the primary cursor's head on the most recent frame, anchored below the
    /// caret (for the completion popup / hover tooltip). `None` before the first render / off-screen.
    pub fn cursor_screen_pos(&self) -> Option<egui::Pos2> {
        let glyph_width = (*self
            .glyph_width_px
            .lock()
            .unwrap_or_else(|e| e.into_inner()))?;
        let (line, col) = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let head = self
                .cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .primary()
                .head;
            byte_to_line_col(head, &buffer)
        };
        let pos = self.screen_pos_for_line_col(line, col, glyph_width)?;
        // Anchor a touch below the caret so the popup does not cover the current line.
        Some(pos + egui::vec2(0.0, 14.0))
    }

    // ── MT-048 Rename Symbol (F2) — public API + triggers ─────────────────────────────────────────

    /// A snapshot of the rename state (the deterministic observation point for the kittest/unit proofs).
    pub fn rename_state(&self) -> RenameState {
        self.rename_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// True while the inline rename input is open (Editing phase).
    pub fn is_rename_input_open(&self) -> bool {
        matches!(
            *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()),
            RenameState::Editing { .. }
        )
    }

    /// True while the multi-file rename preview is shown (Previewing phase).
    pub fn is_rename_preview_open(&self) -> bool {
        matches!(
            *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()),
            RenameState::Previewing { .. }
        )
    }

    /// Set the rename state directly (the deterministic path the tests drive + the off-thread drain uses).
    pub fn set_rename_state(&self, state: RenameState) {
        *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    /// Cancel any rename in progress (Escape / Cancel / focus loss): back to Idle, and clear a pending
    /// off-thread result so a stale preview never lands after cancel.
    pub fn cancel_rename(&self) {
        *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = RenameState::Idle;
        *self.rename_result.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// The rename preview state, or `None` when not in the Previewing phase (the deterministic preview
    /// observation point for the AC-004 proof).
    pub fn rename_preview(&self) -> Option<WorkspaceEditPreview> {
        match &*self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) {
            RenameState::Previewing { workspace_edit } => Some(workspace_edit.clone()),
            _ => None,
        }
    }

    /// MT-048: begin a rename at the primary caret (the F2 keymap dispatch + the context-menu entry both
    /// call this). Resolves the identifier under the cursor via the highlighter's tree-sitter parse tree
    /// (`begin_rename` returns None on a non-identifier — RISK-006, no popup on a keyword/string/space).
    /// On success the rename state becomes `Editing` with the input pre-filled + select-all-on-open armed.
    pub fn begin_rename_at_cursor(&self) {
        let cursor_byte = self.primary_cursor_offset();
        // Ensure the highlight tree reflects the current buffer before resolving (cache hit when unchanged).
        self.ensure_highlight_cache();
        let new_state = {
            let highlighter = self.highlighter.lock().unwrap_or_else(|e| e.into_inner());
            match highlighter.as_ref().and_then(|hl| hl.tree()) {
                Some(tree) => {
                    let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
                    // No backend symbol entity id is resolved here (the local fallback works off the
                    // identifier text); the off-thread request resolves it for the LSP path. Empty is fine.
                    rename::begin_rename(tree, &buffer, cursor_byte, "")
                }
                None => None, // an unhighlighted/plain document has no parse tree -> no tree-sitter rename.
            }
        };
        if let Some(state) = new_state {
            *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
        }
        // On None (non-identifier / no tree) nothing happens — no popup on a non-identifier (RISK-006).
    }

    /// MT-048: confirm the rename (Enter in the inline input). When the draft is empty/whitespace or equals
    /// the original, this is a no-op CANCEL (no rename — the VS Code F2 behavior). Otherwise it spawns the
    /// off-thread LSP-`textDocument/rename`-then-fallback request on `runtime`; the resolved preview is
    /// drained into the Previewing state next frame. The egui thread never blocks (HBR-QUIET).
    pub fn confirm_rename(&self, runtime: &tokio::runtime::Handle) {
        let (original, draft, ident_range) = {
            let guard = self.rename_state.lock().unwrap_or_else(|e| e.into_inner());
            match &*guard {
                RenameState::Editing {
                    original,
                    draft,
                    ident_range,
                    ..
                } => (original.clone(), draft.clone(), ident_range.clone()),
                _ => return, // not editing -> nothing to confirm.
            }
        };
        let new_name = draft.trim().to_owned();
        if new_name.is_empty() || new_name == original {
            // No-op rename: cancel back to Idle (VS Code F2 behavior).
            self.cancel_rename();
            return;
        }

        // Resolve everything the off-thread task needs from the UI thread (no &self capture across .await).
        let uri = self.lsp_uri();
        let position = self.lsp_position_at(ident_range.start);
        let lsp_client = self.lsp_client();
        let buffer_text = self.with_buffer(|b| b.to_string());
        let file_uri = uri
            .clone()
            .unwrap_or_else(|| format!("file:///{}", self.file_path().trim_start_matches('/')));
        let is_open_buffer = true; // the current document is, by definition, an open buffer.
        let occurrence_ranges = self.identifier_occurrences_in_buffer(&original);
        // The set of currently-open buffer URIs, so the preview marks each LSP-edited file open vs to-disk.
        let self_uri = file_uri.clone();
        let cell = Arc::clone(&self.rename_result);

        runtime.spawn(async move {
            // 1) LSP path: issue textDocument/rename over the EXISTING transport (no second transport).
            if let Some(uri_str) = uri.as_deref() {
                match lsp_client.rename(uri_str, position, &new_name).await {
                    Ok(edit) => {
                        // An empty WorkspaceEdit = a no-op rename (the server declined / nothing to change)
                        // OR no server attached (the disabled client returns an empty edit). Distinguish:
                        // when the client is configured + running we trust the empty edit as "no changes";
                        // otherwise fall through to the single-file fallback below.
                        let has_lsp = lsp_client.is_running();
                        let preview = WorkspaceEditPreview::from_lsp(&edit, |u| {
                            if u == self_uri {
                                Some(buffer_text.clone())
                            } else {
                                None // other files are to-disk (read for the preview hunks).
                            }
                        });
                        if has_lsp && !preview.is_empty() {
                            if let Ok(mut slot) = cell.lock() {
                                *slot = Some(Ok(preview));
                            }
                            return;
                        }
                        if has_lsp && preview.is_empty() {
                            // A running server returned no changes: surface "no changes" (empty preview).
                            if let Ok(mut slot) = cell.lock() {
                                *slot = Some(Ok(WorkspaceEditPreview::empty()));
                            }
                            return;
                        }
                        // No running server: fall through to the single-file fallback below.
                    }
                    Err(e) => {
                        if let Ok(mut slot) = cell.lock() {
                            *slot = Some(Err(format!("LSP rename failed: {e}")));
                        }
                        return;
                    }
                }
            }
            // 2) No-LSP single-file fallback (RISK-004 / MC-004 / AC-003): rename only THIS file's
            // occurrences (resolved from tree-sitter — a safe local source), with the banner flag set so
            // the operator is never misled that the rename was project-wide. The references API is NOT
            // consulted for ranges (it has none — the recorded typed blocker); occurrences come from
            // tree-sitter.
            let preview = WorkspaceEditPreview::single_file_fallback(
                file_uri,
                &buffer_text,
                &new_name,
                &occurrence_ranges,
                is_open_buffer,
            );
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(Ok(preview));
            }
        });
    }

    /// MT-048: the in-file occurrence byte ranges of `name` resolved from the highlighter's tree-sitter
    /// parse tree (the SAFE local source for the no-LSP single-file fallback — RISK-006, never a
    /// word-scan). Empty when the document has no parse tree (a plain/unhighlighted document).
    fn identifier_occurrences_in_buffer(&self, name: &str) -> Vec<std::ops::Range<usize>> {
        let highlighter = self.highlighter.lock().unwrap_or_else(|e| e.into_inner());
        match highlighter.as_ref().and_then(|hl| hl.tree()) {
            Some(tree) => {
                let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
                rename::identifier_occurrences(tree, &buffer, name)
            }
            None => Vec::new(),
        }
    }

    /// MT-048: apply the current rename preview (the Apply button / the swarm Apply node). Applies the
    /// edits to the open buffer (this document) in DESCENDING offset order (RISK-001) and writes any
    /// to-disk file atomically (RISK-002). On success the buffer is updated, the document re-highlighted,
    /// and the rename returns to Idle. On failure the state becomes `Error` with the message (and the
    /// already-applied files stay applied — the truthful partial report). Returns the apply report on
    /// success. The current document's URI is matched against the preview's open-buffer files so the
    /// in-memory `TextBuffer` is the apply target for this file.
    pub fn apply_rename_preview(&self) -> Option<rename::RenameApplyReport> {
        let preview = self.rename_preview()?;
        let self_uri = self
            .lsp_uri()
            .unwrap_or_else(|| format!("file:///{}", self.file_path().trim_start_matches('/')));
        // Apply: open-buffer files for THIS document route to the in-memory TextBuffer; any other
        // open-buffer URI is unknown to this panel (a multi-pane host would route it — out of this MT's
        // single-panel scope), so it reads the panel's buffer only for the self uri.
        let buffer_text = self.with_buffer(|b| b.to_string());
        let mut new_self_text: Option<String> = None;
        let result = rename::apply_preview(
            &preview,
            |uri| {
                if uri == self_uri {
                    Some(buffer_text.clone())
                } else {
                    // Another open buffer in a multi-pane host; this single-panel MT does not own it, so
                    // read it from disk as a to-disk file would be — but the preview already marked it
                    // open. Returning None makes apply treat it as to-disk (read from disk). For the
                    // common single-file + cross-file-to-disk rename this is correct.
                    None
                }
            },
            |uri, text| {
                if uri == self_uri {
                    new_self_text = Some(text.to_owned());
                }
            },
        );
        match result {
            Ok(report) => {
                // Install the renamed text back into THIS document's buffer + re-highlight (AC-002).
                if let Some(text) = new_self_text {
                    self.set_text(&text);
                    self.record_code_edit_mutation_text(&buffer_text, &text);
                }
                *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = RenameState::Idle;
                Some(report)
            }
            Err(e) => {
                *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = RenameState::Error {
                    message: e.to_string(),
                };
                None
            }
        }
    }

    /// MT-048: drain a delivered off-thread rename result into the rename state (called each frame). An
    /// `Ok(preview)` becomes `Previewing` (or stays Idle on an empty no-op preview with a trace); an
    /// `Err(message)` becomes `Error`. A no-op when no result is pending.
    fn drain_rename_result(&self) {
        let pending = self
            .rename_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(result) = pending {
            match result {
                Ok(preview) if preview.is_empty() => {
                    // No changes (the server declined / nothing to rename): return to Idle silently.
                    *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) =
                        RenameState::Idle;
                    tracing::debug!("code editor: rename produced no changes");
                }
                Ok(preview) => {
                    *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) =
                        RenameState::Previewing {
                            workspace_edit: preview,
                        };
                }
                Err(message) => {
                    *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) =
                        RenameState::Error { message };
                }
            }
        }
    }

    // ── MT-049 Code actions / quick fixes (the lightbulb) — public API + triggers ─────────────────

    /// True when the quick-fix popup menu is currently open (the deterministic observation point for the
    /// AC-005 interaction proof).
    pub fn is_quickfix_menu_open(&self) -> bool {
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_menu_open()
    }

    /// True when line `line` carries at least one available code action (drives the gutter lightbulb —
    /// AC-003 / AC-006). False while idle / on a line with no actions / with no LSP attached.
    pub fn has_quickfix_on_line(&self, line: usize) -> bool {
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .has_actions_on_line(line)
    }

    /// The titles of the actions currently in the quick-fix list (the deterministic observation point for
    /// the AC-001/AC-004 proofs). Empty when idle / no actions.
    pub fn quickfix_action_titles(&self) -> Vec<String> {
        let guard = self
            .code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match guard.state() {
            Some(s) => s.actions.iter().map(|a| a.title.clone()).collect(),
            None => Vec::new(),
        }
    }

    /// Actual quick-fix lightbulb draw positions from the most recent frame, for regression tests.
    #[doc(hidden)]
    pub fn quickfix_lightbulb_positions_for_test(&self) -> Vec<(usize, egui::Pos2)> {
        self.last_quickfix_lightbulbs
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Install a resolved action list directly (the deterministic path the kittest/unit proofs use, the
    /// same way `open_signature_help` feeds synthetic state). `open_menu` opens the popup immediately.
    pub fn set_quickfix_actions(
        &self,
        line: usize,
        actions: Vec<code_actions::CodeActionItem>,
        open_menu: bool,
    ) {
        let version = self.buffer_version.load(Ordering::Relaxed);
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_actions(line, version, actions, open_menu);
    }

    /// Close the quick-fix menu (Escape / apply / focus loss).
    pub fn close_quickfix_menu(&self) {
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .close_menu();
    }

    /// Clear the quick-fix controller to idle (no lightbulb, no menu).
    pub fn clear_quickfix(&self) {
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
    }

    /// Set the quick-fix cursor-rest debounce threshold (a kittest sets it to ZERO so the rest crossing
    /// fires on the first settled frame — the same deterministic-dwell hook the MT-034 note-refs path uses).
    pub fn set_quickfix_rest_threshold(&self, threshold: std::time::Duration) {
        *self
            .code_action_rest_threshold
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = threshold;
    }

    /// Whether the Ctrl+. quick-fix request is currently armed (not yet consumed by the pump). The
    /// live-path test reads it to prove the pump CONSUMED the arm in the same frame.
    pub fn quick_fix_request_armed_for_test(&self) -> bool {
        self.quick_fix_request.load(Ordering::Relaxed)
    }

    /// Monotonic proof that a request reached the real quick-fix handler, not merely its transient arm.
    pub fn quick_fix_request_generation_for_test(&self) -> u64 {
        self.quick_fix_request_generation.load(Ordering::Relaxed)
    }

    /// Concrete last request state captured at the same handler boundary as the generation increment.
    pub fn last_quick_fix_request_for_test(&self) -> Option<(usize, u64, bool)> {
        *self
            .last_quick_fix_request
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    /// Whether `line` carries at least one diagnostic in the MT-007 gutter diagnostic store (the gate for
    /// the cursor-rest code-action request — RISK-001 / MC-001: only query the server on a diagnostic line).
    fn line_has_diagnostic(&self, line: usize) -> bool {
        self.diagnostic_markers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|m| m.line == line && matches!(m.kind, GutterMarkerKind::Diagnostic(_)))
    }

    /// The LSP diagnostics on `line` (the `context.diagnostics` the `textDocument/codeAction` request
    /// carries, so the server can scope its quick fixes to those diagnostics). Built from the MT-007 gutter
    /// store: each diagnostic marker on `line` becomes an `lsp_types::Diagnostic` covering the whole line
    /// (the gutter store is line-granular — the same line-level shape MT-007 records).
    fn lsp_diagnostics_on_line(&self, line: usize) -> Vec<lsp_types::Diagnostic> {
        let markers = self
            .diagnostic_markers
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        markers
            .iter()
            .filter(|m| m.line == line && matches!(m.kind, GutterMarkerKind::Diagnostic(_)))
            .map(|m| {
                let severity = match &m.kind {
                    GutterMarkerKind::Diagnostic(DiagnosticSeverity::Error) => {
                        Some(lsp_types::DiagnosticSeverity::ERROR)
                    }
                    GutterMarkerKind::Diagnostic(DiagnosticSeverity::Warning) => {
                        Some(lsp_types::DiagnosticSeverity::WARNING)
                    }
                    GutterMarkerKind::Diagnostic(DiagnosticSeverity::Info) => {
                        Some(lsp_types::DiagnosticSeverity::INFORMATION)
                    }
                    GutterMarkerKind::Diagnostic(DiagnosticSeverity::Hint) => {
                        Some(lsp_types::DiagnosticSeverity::HINT)
                    }
                    _ => None,
                };
                lsp_types::Diagnostic {
                    range: self.line_lsp_range(line),
                    severity,
                    message: m.message.clone(),
                    ..Default::default()
                }
            })
            .collect()
    }

    /// The LSP `Range` covering the whole of buffer `line` (the code-action request range + the
    /// per-diagnostic range). 0-based start of `line` to the start of `line + 1` (clamped at EOF).
    fn line_lsp_range(&self, line: usize) -> lsp_types::Range {
        let (start_char, end_char) = self.with_buffer(|b| {
            let start = b.line_to_byte(line).unwrap_or(0);
            let end = b.line_to_byte(line + 1).unwrap_or_else(|| b.len_bytes());
            (0u32, end.saturating_sub(start) as u32)
        });
        lsp_types::Range {
            start: lsp_types::Position {
                line: line as u32,
                character: start_char,
            },
            end: lsp_types::Position {
                line: line as u32,
                character: end_char,
            },
        }
    }

    /// MT-049: spawn the off-thread `textDocument/codeAction` request for `line` and deliver the normalized
    /// actions over the result channel (drained next frame by [`pump_code_actions`](Self::pump_code_actions)).
    /// `open_menu` opens the menu when the result lands (the Ctrl+. / lightbulb / context-menu path) vs only
    /// lighting the bulb (the passive cursor-rest path). The egui thread never blocks (HBR-QUIET): the LSP
    /// request runs on the injected runtime. When no LSP is attached the request returns an empty action
    /// list (graceful — AC-006), which still lands so the Ctrl+. degraded menu can show "No quick fixes".
    pub fn trigger_quick_fix(
        &self,
        runtime: &tokio::runtime::Handle,
        line: usize,
        open_menu: bool,
    ) {
        let version = self.buffer_version.load(Ordering::Relaxed);
        self.quick_fix_request_generation
            .fetch_add(1, Ordering::Relaxed);
        *self
            .last_quick_fix_request
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((line, version, open_menu));
        // Mark the request in flight so the debounce guard does not fire a second one (RISK-001 / MC-001).
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .mark_request_in_flight(line, version);

        let Some(uri) = self.lsp_uri() else {
            // No file URI -> no LSP request possible; deliver an empty list so the degraded menu shows
            // "No quick fixes available" when open_menu is set (AC-006 — never a panic).
            let _ = self.code_action_tx.send(code_actions::CodeActionResult {
                line,
                buffer_version: version,
                actions: Vec::new(),
                open_menu,
            });
            return;
        };
        let range = self.line_lsp_range(line);
        let diagnostics = self.lsp_diagnostics_on_line(line);
        let lsp_client = self.lsp_client();
        let tx = self.code_action_tx.clone();

        runtime.spawn(async move {
            // The CodeActionContext scopes the request to the line's diagnostics; `only: None` lets the
            // server return any kind (quickfix/refactor/source). A no-server client returns empty (AC-006).
            let context = lsp_types::CodeActionContext {
                diagnostics,
                only: None,
                trigger_kind: None,
            };
            let response = lsp_client.code_action(&uri, range, context).await;
            let actions = code_actions::normalize_code_actions(response);
            // Deliver the result (empty or not) so the drain installs it; a send error (the panel dropped)
            // is a benign no-op.
            let _ = tx.send(code_actions::CodeActionResult {
                line,
                buffer_version: version,
                actions,
                open_menu,
            });
        });
    }

    /// MT-049: the per-frame quick-fix pump (called from `show` AFTER the cursor input so the caret line is
    /// current). It (1) installs the result receiver on the controller on the first frame, (2) drains any
    /// delivered result, (3) fires a Ctrl+. / context-menu request when one is armed, and (4) advances the
    /// cursor-rest debounce and fires a passive request when the caret has rested ~300ms on a diagnostic
    /// line (RISK-001 / MC-001 — never per idle frame; cancel on a line change). A graceful no-op without an
    /// injected runtime (a headless harness drives the deterministic `set_quickfix_actions` path instead).
    fn pump_code_actions(&self) {
        // (1) Install the result receiver on the controller once (one consumer per channel).
        if let Some(rx) = self
            .code_action_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            self.code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .set_result_receiver(rx);
        }
        // (2) Drain any delivered result into the controller state (lights the bulb / opens the menu).
        self.code_action_controller
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .poll_results();

        let Some(runtime) = self.runtime_handle() else {
            // No runtime: the off-thread request path is unavailable. Consume any Ctrl+. arm so it does not
            // linger, and skip the cursor-rest trigger (the deterministic tests drive set_quickfix_actions).
            self.quick_fix_request.store(false, Ordering::Relaxed);
            return;
        };

        let cursor_line = self.primary_cursor_line();

        // (3) Ctrl+. / context-menu arm: fire a request for the caret line and OPEN the menu immediately.
        if self.quick_fix_request.swap(false, Ordering::Relaxed) {
            self.trigger_quick_fix(&runtime, cursor_line, /* open_menu */ true);
            return; // do not also fire the passive cursor-rest request this frame.
        }

        // (4) Passive cursor-rest trigger: only on a diagnostic line, only once the caret has rested past
        // the debounce window, and only when a request is not already in flight for this line (RISK-001 /
        // MC-001 — no per-idle-frame server flood; cancel/restart the dwell on a line change).
        let threshold = *self
            .code_action_rest_threshold
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let now = std::time::Instant::now();
        let mut rest = self
            .code_action_rest
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let on_diagnostic_line = self.line_has_diagnostic(cursor_line);
        if !on_diagnostic_line {
            // Off a diagnostic line: reset the dwell and clear any stale actions for a now-irrelevant line.
            *rest = None;
            let mut controller = self
                .code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if controller
                .active_line()
                .map(|l| l != cursor_line)
                .unwrap_or(false)
                && !controller.is_menu_open()
            {
                controller.clear();
            }
            return;
        }
        // On a diagnostic line: advance / restart the dwell clock for this line.
        let crossed = match *rest {
            Some((line, since)) if line == cursor_line => now.duration_since(since) >= threshold,
            _ => {
                *rest = Some((cursor_line, now));
                threshold.is_zero() // a zero threshold fires on the first settled frame (the kittest hook).
            }
        };
        if crossed {
            // Fire ONCE per rest: clear the dwell so the next frame does not re-fire, and skip when a
            // request for this line is already in flight or actions are already loaded (RISK-001).
            let already = {
                let controller = self
                    .code_action_controller
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                controller.request_in_flight()
                    || controller.active_line() == Some(cursor_line)
                        && controller.has_actions_on_line(cursor_line)
            };
            *rest = Some((cursor_line, now)); // keep the line anchored so a re-rest does not immediately re-fire.
            if !already {
                drop(rest); // release before the trigger locks the controller.
                self.trigger_quick_fix(&runtime, cursor_line, /* open_menu */ false);
            }
        }
    }

    /// MT-049: apply the SELECTED quick-fix action — the menu's Enter / a row click / the swarm Apply node.
    /// DELEGATES the in-file WorkspaceEdit apply to the MT-048 path via the controller's `apply_selected`
    /// (RISK-002 / MC-002 / AC-002 — no re-implementation). Cross-file edits are routed through MT-048's
    /// [`rename::apply_preview`] multi-file/atomic path (RISK-005 / MC-005 — cross-file fixes apply, not
    /// dropped). A command-only action is routed through `workspace/executeCommand` off-thread (RISK-003 /
    /// MC-003 — graceful no-op if the server cannot execute it). On a stale buffer the apply is rejected and
    /// re-requested (RISK-007 / MC-007). Returns the applied-action outcome (or `None` on a reject).
    pub fn apply_quickfix(&self) -> Option<AppliedAction> {
        let self_uri = self
            .lsp_uri()
            .unwrap_or_else(|| format!("file:///{}", self.file_path().trim_start_matches('/')));
        let live_version = self.buffer_version.load(Ordering::Relaxed);

        // Apply against a working copy of the buffer + cursors (so the MT-048 apply path mutates them), then
        // install the result back into the panel + re-highlight (AC-002). Holding the controller + buffer
        // locks across the MT-048 apply is safe (no egui calls inside).
        let outcome = {
            let mut controller = self
                .code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let mut buffer = self
                .buffer
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            let mut cursors = self
                .cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            match controller.apply_selected(&mut buffer, &mut cursors, &self_uri, live_version) {
                Ok(applied) => Some((applied, buffer.to_string())),
                Err(code_actions::CodeActionError::StaleBuffer) => {
                    // RISK-007 / MC-007: the buffer changed; re-request for the same line and reject the apply.
                    let line = controller.active_line();
                    drop(controller);
                    if let (Some(line), Some(rt)) = (line, self.runtime_handle()) {
                        self.trigger_quick_fix(&rt, line, true);
                    }
                    return None;
                }
                Err(_) => {
                    // A bad range / no-such-action: close the menu, report nothing applied (no panic).
                    controller.close_menu();
                    return None;
                }
            }
        };

        let (applied, new_text) = outcome?;
        match &applied {
            AppliedAction::Edit { cross_file, .. } => {
                // Install the in-file result back into the panel buffer + re-highlight (AC-002).
                let before = self.buffer().to_string();
                self.set_text(&new_text);
                self.record_code_edit_mutation_text(&before, &new_text);
                // Route any cross-file edits through MT-048's multi-file/atomic apply (RISK-005 / MC-005).
                // The result MUST be surfaced, NEVER discarded: the in-file edit already committed via
                // `set_text` above, so a cross-file to-disk write that fails (missing/locked file, a stale
                // BadRange) would otherwise leave the workspace half-applied with NO operator-visible signal.
                // Bind the Result, log it (warn on Err naming the cross-file URI + partial report; debug on
                // Ok with the file/edit count), and record it on the typed cell so a swarm agent / a test can
                // observe the cross-file outcome (MC-005 — surface/log, do not silently drop).
                if !cross_file.files.is_empty() {
                    let outcome = rename::apply_preview(
                        cross_file,
                        |_uri| None, // cross-file targets are to-disk (read inside apply_preview).
                        |_uri, _text| {},
                    );
                    match &outcome {
                        Ok(report) => {
                            tracing::debug!(
                                files_changed = report.files_changed.len(),
                                edits_applied = report.edits_applied,
                                "code editor: quick-fix cross-file edits applied"
                            );
                        }
                        Err(e) => {
                            // A partial cross-file failure: the active buffer already changed, so this is an
                            // inconsistent on-disk vs in-buffer state the operator MUST be able to see.
                            let partial = match e {
                                rename::RenameError::Io { partial, .. }
                                | rename::RenameError::BadRange { partial, .. } => partial,
                            };
                            tracing::warn!(
                                error = %e,
                                cross_file_files_applied = partial.files_changed.len(),
                                in_file_already_applied = true,
                                "code editor: quick-fix cross-file apply FAILED — workspace is partially \
                                 applied (in-file edit committed, a cross-file write did not); not silently \
                                 dropped (MC-005)"
                            );
                        }
                    }
                    *self
                        .last_quickfix_cross_file
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) =
                        Some(outcome.map_err(|e| e.to_string()));
                }
            }
            AppliedAction::Command { command } => {
                // Route a command-only action through workspace/executeCommand off-thread (RISK-003 /
                // MC-003 — graceful no-op if unsupported). The server then pushes workspace/applyEdit which
                // the diagnostics/edit path handles; no in-file mutation here.
                if let Some(rt) = self.runtime_handle() {
                    let lsp_client = self.lsp_client();
                    let cmd = command.command.clone();
                    let args = command.arguments.clone();
                    rt.spawn(async move {
                        let _ = lsp_client.execute_command(&cmd, &args).await;
                    });
                }
            }
            AppliedAction::NoOp => {}
        }
        Some(applied)
    }

    /// MT-049 (RISK-005 / MC-005): the LAST cross-file quick-fix apply outcome recorded by
    /// [`apply_quickfix`](Self::apply_quickfix). `None` until a chosen action with cross-file edits has been
    /// applied; thereafter `Ok(report)` when every cross-file write succeeded, or `Err(message)` (the
    /// `RenameError` text naming the failing URI) when a cross-file to-disk write failed — surfaced, never
    /// silently dropped. The observation point a swarm agent / a unit test reads to prove the cross-file
    /// error path is taken (the in-file edit still applies regardless).
    pub fn last_quickfix_cross_file_result(&self) -> Option<Result<RenameApplyReport, String>> {
        self.last_quickfix_cross_file
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    // ── MT-050 Format Document / Format Selection — public API + triggers ──────────────────────────

    /// MT-050: the buffer's resolved language id (e.g. `"rust"`), the gate `formatter_available` consults.
    /// Empty for a plain/unhighlighted document (no grammar -> no formatter).
    pub fn language_id(&self) -> String {
        self.language_id.to_owned()
    }

    /// MT-050: the document URI for a formatting request (`file://`), or `None` for an in-memory buffer with
    /// no file path. Reuses the MT-047/048 `lsp_uri` mapping; falls back to a `file:///<path>` form so a
    /// test with a bare relative path still yields a URI the request can carry.
    pub fn format_uri(&self) -> Option<String> {
        self.lsp_uri().or_else(|| {
            let path = self.file_path();
            if path.trim().is_empty() {
                None
            } else {
                Some(format!("file:///{}", path.trim_start_matches('/')))
            }
        })
    }

    /// MT-050: the primary selection as a `(start, end)` BYTE range. A collapsed caret yields
    /// `(caret, caret)`; `formatting::selection_range_for` then maps an empty range to the current line.
    pub fn primary_selection_bytes(&self) -> (usize, usize) {
        let primary = self.cursors().primary();
        let r = primary.range();
        let len = self.with_buffer(|b| b.len_bytes());
        let end = r.end.min(len);
        let start = r.start.min(end);
        (start, end)
    }

    /// MT-050: whether a formatter is available for this buffer (an LSP attached + the server advertised
    /// `documentFormattingProvider`). Drives the EDIT-menu / context-menu enabled state + the keymap
    /// no-op gate (AC-003). The `&self` convenience over `formatting::formatter_available`.
    pub fn formatter_available(&self) -> bool {
        let lsp = self.lsp_client();
        formatting::formatter_available(&lsp, &self.language_id())
    }

    /// MT-050: the format menu descriptors (EDIT-menu + context-menu Format Document / Format Selection),
    /// each reflecting the live enabled/disabled state (RISK-007 — the menu builders consume these rather
    /// than this MT forking a menu file). The host menu builders render each descriptor as an enabled item
    /// (dispatching the format action) or a disabled item (greyed + the no-formatter tooltip + AccessKit
    /// disabled node).
    pub fn format_menu_descriptors(&self) -> [formatting::FormatMenuDescriptor; 3] {
        let lsp = self.lsp_client();
        formatting::menu_descriptors(&lsp, &self.language_id())
    }

    /// MT-050: the LAST format toast (the non-blocking LspError / NoFormatter surface — AC-006), or `None`.
    /// Queryable by a swarm agent + the unit tests to prove the error path surfaces a toast (not a panic /
    /// a blocking dialog).
    pub fn last_format_toast(&self) -> Option<String> {
        self.last_format_toast
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// TEST/dispatch hook: whether the Format Document request is currently armed (drained by the pump).
    pub fn format_request_armed_for_test(&self) -> bool {
        self.format_document_request.load(Ordering::Relaxed)
            || self.format_selection_request.load(Ordering::Relaxed)
    }

    /// MT-050: arm a Format Document request (the Alt+Shift+F keymap / EDIT-menu / context-menu entry). When
    /// no formatter is available this is a NO-OP that records the no-formatter toast (AC-003 — never a
    /// panic, never a frame block); otherwise the per-frame pump fires the off-thread request and applies
    /// the result. Armed here (not run mid-key-dispatch) so the request runs on the pump with the live
    /// runtime — the same arm-then-pump discipline MT-049's Ctrl+. uses.
    pub fn request_format_document(&self) {
        if !self.formatter_available() {
            // The disabled keymap path: a no-op + a (queryable) toast, no panic, no frame block (AC-003).
            *self
                .last_format_toast
                .lock()
                .unwrap_or_else(|e| e.into_inner()) =
                Some(formatting::NO_FORMATTER_TOOLTIP.to_owned());
            return;
        }
        self.format_document_request.store(true, Ordering::Relaxed);
    }

    /// MT-050: arm a Format Selection request (the context-menu 'Format Selection' entry / AccessKit node).
    /// Same gating + arm-then-pump discipline as [`request_format_document`], using
    /// `documentRangeFormattingProvider` as the gate.
    pub fn request_format_selection(&self) {
        let lsp = self.lsp_client();
        if !formatting::range_formatter_available(&lsp, &self.language_id()) {
            *self
                .last_format_toast
                .lock()
                .unwrap_or_else(|e| e.into_inner()) =
                Some(formatting::NO_FORMATTER_TOOLTIP.to_owned());
            return;
        }
        self.format_selection_request.store(true, Ordering::Relaxed);
    }

    /// MT-050 per-frame format pump (called from the code-intelligence pump). Drains any delivered format
    /// result (installing the formatted text as ONE undo step + surfacing the error toast), then, if a
    /// format request is armed this frame, fires the off-thread `textDocument/formatting` /
    /// `rangeFormatting` request. The egui thread NEVER blocks on the LSP (HBR-QUIET / RISK-005): the
    /// request runs on the injected runtime and writes its typed outcome to the delivery cell for the next
    /// frame's drain. A no-op without a runtime (the deterministic tests drive
    /// [`formatting::resolve_format_outcome`] directly + the kittest drives the live async pump).
    fn pump_formatting(&self) {
        // (1) Drain any delivered off-thread format result.
        self.drain_format_result();

        let Some(runtime) = self.runtime_handle() else {
            // No runtime: clear any arm so it does not linger (the synthetic apply path still works).
            self.format_document_request.store(false, Ordering::Relaxed);
            self.format_selection_request
                .store(false, Ordering::Relaxed);
            return;
        };

        let want_doc = self.format_document_request.swap(false, Ordering::Relaxed);
        let want_sel = self.format_selection_request.swap(false, Ordering::Relaxed);
        if !want_doc && !want_sel {
            return;
        }

        // Resolve everything the off-thread task needs from the UI thread (no &self capture across .await).
        let Some(uri) = self.format_uri() else { return };
        let lsp_client = self.lsp_client();
        let options = formatting::default_formatting_options();
        let before = self.with_buffer(|b| b.to_string());
        let cell = Arc::clone(&self.format_result);

        if want_doc {
            runtime.spawn(async move {
                let outcome = match lsp_client.format_document(&uri, options).await {
                    Ok(edits) => formatting::resolve_format_outcome(&before, &edits),
                    Err(e) => (
                        None,
                        FormatOutcome::LspError(format!("Formatting failed: {e}")),
                    ),
                };
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some((before, outcome.0, outcome.1));
                }
            });
        } else {
            // Format Selection: compute the UTF-16-correct range from the current selection on the UI thread.
            let (start, end) = self.primary_selection_bytes();
            let range = {
                let buffer = self.buffer();
                formatting::selection_range_for(&buffer, start, end)
            };
            let Some(range) = range else { return };
            runtime.spawn(async move {
                let outcome = match lsp_client.format_range(&uri, range, options).await {
                    Ok(edits) => formatting::resolve_format_outcome(&before, &edits),
                    Err(e) => (
                        None,
                        FormatOutcome::LspError(format!("Formatting failed: {e}")),
                    ),
                };
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some((before, outcome.0, outcome.1));
                }
            });
        }
    }

    /// MT-050: drain a delivered off-thread format result into the buffer (called each frame). On an
    /// `Applied` outcome with formatted text the text is installed via `set_text` (re-clamping the cursor —
    /// RISK-006) and the single undo entry is recorded through the host bus (AC-001). On `LspError` /
    /// `NoFormatter` the toast surface is set (AC-006). `NoChange` is silent. A no-op when nothing pending.
    fn drain_format_result(&self) {
        let pending = self
            .format_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        let Some((before, formatted, outcome)) = pending else {
            return;
        };
        match &outcome {
            FormatOutcome::Applied { edit_count } => {
                if let Some(after) = formatted {
                    if after != before {
                        // Install the formatted text (re-clamps the cursor — RISK-006) and record ONE undo
                        // entry (before -> after) so a single Ctrl+Z reverts the WHOLE format (AC-001).
                        self.set_text(&after);
                        self.record_format_undo(&before, &after);
                        tracing::debug!(
                            "code editor: formatted document, {edit_count} edits (single undo)"
                        );
                    }
                }
            }
            FormatOutcome::NoChange => {
                tracing::debug!("code editor: format produced no changes (already formatted)");
            }
            FormatOutcome::NoFormatter => {
                *self
                    .last_format_toast
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) =
                    Some(formatting::NO_FORMATTER_TOOLTIP.to_owned());
            }
            FormatOutcome::LspError(msg) => {
                // A non-blocking toast (NOT a frame-blocking dialog — AC-006 / MC-006). Surfaced + logged.
                tracing::warn!("code editor: format failed: {msg}");
                *self
                    .last_format_toast
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(msg.clone());
            }
        }
    }

    /// MT-050: queue the SINGLE undo snapshot for a format (before -> after whole-buffer text) so the host
    /// factory render records ONE undo entry through the unified-undo bus next frame, so a single Ctrl+Z
    /// reverts the entire format (AC-001). The panel itself does NOT hold the bus / pane id / its own
    /// `Arc` self-handle (those live at the factory render boundary where every code edit's undo is
    /// recorded — the wrap-not-fork discipline), so the panel records the (before, after) pair here and
    /// [`CodeEditorPaneFactory::render`] drains it into `interop_adapter::push_code_edit_undo`. Only the
    /// LATEST format's snapshot is kept (a second format before the drain supersedes the first — the
    /// host applies them in order, so the newest before/after pair is the correct single entry to push).
    fn record_format_undo(&self, before: &str, after: &str) {
        if !self.record_code_edit_mutation_text(before, after) {
            return;
        }
        *self
            .pending_format_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((before.to_owned(), after.to_owned()));
    }

    /// MT-050: take the queued format undo snapshot (before, after) the factory render pushes onto the
    /// shared unified-undo bus. `None` when no format applied since the last drain. The factory drains this
    /// each frame so the single undo entry is recorded at the SAME boundary every code edit's undo is.
    pub fn take_pending_format_undo(&self) -> Option<(String, String)> {
        self.pending_format_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    fn record_text_edit_undo(
        &self,
        before: TextBuffer,
        after: TextBuffer,
        description: &'static str,
    ) {
        if !self.record_code_edit_mutation(&before, &after) {
            return;
        }
        let (batch_before, replace_tail) = self
            .text_edit_undo_batcher
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .observe_edit(before, Instant::now());
        *self
            .pending_text_edit_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(PendingCodeTextUndo {
            before: batch_before,
            after,
            description,
            replace_tail,
        });
    }

    /// Record one successful code-buffer mutation for the MT-036 Flight Recorder producer. The return
    /// value is `false` for a byte-for-byte no-op, allowing callers that share this gate with undo staging
    /// to reject no-op receipts and no-op undo entries together.
    fn record_code_edit_mutation(&self, before: &TextBuffer, after: &TextBuffer) -> bool {
        // Ordinary typing/deletion changes byte length, so the hot path never stringifies the rope. Only
        // a same-size replacement needs the exact fallback comparison to reject a byte-for-byte no-op.
        if before.len_bytes() == after.len_bytes() && before.to_string() == after.to_string() {
            return false;
        }
        self.record_code_edit_line_delta(before.len_lines(), after.len_lines());
        true
    }

    fn record_code_edit_mutation_text(&self, before: &str, after: &str) -> bool {
        if before == after {
            return false;
        }
        let line_count = |text: &str| {
            text.as_bytes()
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                .saturating_add(1)
        };
        self.record_code_edit_line_delta(line_count(before), line_count(after));
        true
    }

    fn record_code_edit_line_delta(&self, before_lines: usize, after_lines: usize) {
        let line_delta = i64::try_from(after_lines)
            .unwrap_or(i64::MAX)
            .saturating_sub(i64::try_from(before_lines).unwrap_or(i64::MAX));
        let pane_id = self
            .host_render_pane_id
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone();
        let workspace_id = self.workspace_id();
        let file_path = self.file_path();
        let mut receipts = self
            .pending_code_edit_receipts
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if let Some(last) = receipts.back_mut().filter(|last| {
            last.pane_id == pane_id
                && last.workspace_id == workspace_id
                && last.file_path == file_path
        }) {
            last.line_delta = last.line_delta.saturating_add(line_delta);
        } else {
            receipts.push_back(PendingCodeEditMutationReceipt {
                line_delta,
                pane_id,
                workspace_id,
                file_path,
            });
        }
    }

    fn take_pending_code_edit_receipts(&self) -> Vec<PendingCodeEditMutationReceipt> {
        self.pending_code_edit_receipts
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .drain(..)
            .collect()
    }

    fn apply_text_edit_undoable(
        &self,
        description: &'static str,
        edit: impl FnOnce(&Self) -> usize,
    ) -> usize {
        let before = self.buffer();
        let applied = edit(self);
        if applied > 0 {
            self.record_text_edit_undo(before, self.buffer(), description);
        }
        applied
    }

    fn has_pending_text_edit_undo(&self) -> bool {
        self.pending_text_edit_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    fn take_pending_text_edit_undo(&self) -> Option<PendingCodeTextUndo> {
        self.pending_text_edit_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    pub fn reset_text_edit_undo_batch(&self) {
        self.text_edit_undo_batcher
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .reset();
        *self
            .pending_text_edit_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// WP-KERNEL-012 MT-035 FIX A (undo-history corruption): clear ONLY the typing-coalescer timing so the
    /// NEXT live keystroke starts a FRESH undo entry instead of `replace_tail`-ing over a NON-typing tail
    /// (format / line-op / cut / paste). Unlike [`reset_text_edit_undo_batch`], this deliberately does NOT
    /// clear `pending_text_edit_undo`: a typing entry staged this frame but not yet drained (bus contended)
    /// must survive so a non-typing push in the SAME frame cannot silently drop it. Called from
    /// `interop_adapter::push_code_edit_undo` — the single boundary EVERY non-typing code-edit undo entry is
    /// pushed through — so a `type -> format -> type` burst inside the 500ms window keeps all three entries.
    pub fn reset_text_edit_undo_batch_timing(&self) {
        self.text_edit_undo_batcher
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .reset();
    }

    // ── MT-051 line-edit buffer transforms — settings + dispatch + single-undo ─────────────────────────

    /// MT-051: set the operator's indent settings (`editor.tabSize` + `editor.insertSpaces`) so the
    /// line-edit transforms use them instead of a hardcoded 4 (MC-006 — RISK-006). The host plumbs these
    /// from the editor-settings layer; `tab_size` is clamped to >= 1 (a 0-width indent unit is invalid).
    pub fn set_indent_settings(&self, tab_size: usize, insert_spaces: bool) {
        self.tab_size
            .store(tab_size.max(1) as u64, Ordering::Relaxed);
        self.insert_spaces.store(insert_spaces, Ordering::Relaxed);
    }

    /// MT-051: the current indent settings `(tab_size, insert_spaces)` (for tests / the host).
    pub fn indent_settings(&self) -> (usize, bool) {
        (
            self.tab_size.load(Ordering::Relaxed) as usize,
            self.insert_spaces.load(Ordering::Relaxed),
        )
    }

    // ── MT-071 file-metadata API (status-bar segments: language / EOL / indent / encoding / whitespace) ──

    /// MT-071: the current indent style as a typed [`IndentStyle`](super::file_meta::IndentStyle),
    /// derived from the REUSED MT-051 `(tab_size, insert_spaces)` slot (not a parallel store — RISK-004).
    /// The Indent status-bar segment reads this; "Convert indentation" / "Change tab size" write it back
    /// via [`set_indent_style`](Self::set_indent_style).
    pub fn indent_style(&self) -> super::file_meta::IndentStyle {
        let (size, insert_spaces) = self.indent_settings();
        let kind = if insert_spaces {
            super::file_meta::IndentKind::Spaces
        } else {
            super::file_meta::IndentKind::Tabs
        };
        super::file_meta::IndentStyle { kind, size }
    }

    /// MT-071: set the active indent style (tabs-vs-spaces + size). Writes the REUSED MT-051 indent slot
    /// so the Tab key's editing behavior follows immediately (AC-003) — Tabs inserts a literal tab,
    /// Spaces inserts `size` spaces. No parallel store (RISK-004/MC-004).
    pub fn set_indent_style(&self, style: super::file_meta::IndentStyle) {
        let insert_spaces = matches!(style.kind, super::file_meta::IndentKind::Spaces);
        self.set_indent_settings(style.size, insert_spaces);
    }

    /// MT-071: the document's active line-ending style (LF / CRLF). The status-bar EOL segment reads it.
    pub fn eol(&self) -> super::file_meta::Eol {
        *self.eol.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// MT-071: convert the document's line endings to `target` as EXACTLY ONE undo step
    /// (RISK-002/MC-002). Rewrites the WHOLE buffer through [`set_text`](Self::set_text) — the same
    /// whole-buffer replace the MT-035/050 single-undo path uses — and queues ONE `(description, before,
    /// after)` snapshot into the line-op undo slot the factory render drains into
    /// `interop_adapter::push_code_edit_undo`, so a single Ctrl+Z reverts the ENTIRE conversion at the
    /// SAME unified-undo bus boundary every code edit records at (no per-line edits, no parallel undo
    /// stack). Records the new EOL on the doc model. Returns `true` when the buffer text changed (a no-op
    /// when the document is already in the target EOL, so re-running is idempotent).
    pub fn convert_eol(&self, target: super::file_meta::Eol) -> bool {
        let before = self.buffer().to_string();
        let after = target.rewrite(&before);
        *self.eol.lock().unwrap_or_else(|e| e.into_inner()) = target;
        if after == before {
            return false;
        }
        // ONE whole-buffer replace = one undo step. Queue the before/after snapshot so the factory render
        // records it as a SINGLE unified-undo entry (the MT-035/051 single-undo bus boundary).
        self.set_text(&after);
        self.record_code_edit_mutation_text(&before, &after);
        *self
            .pending_line_op_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(("Convert Line Endings", before, after));
        true
    }

    /// MT-071: the document's active text encoding (default UTF-8). The status-bar encoding segment
    /// reads it.
    pub fn encoding(&self) -> super::file_meta::Encoding {
        *self.encoding.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// MT-071: re-decode the document's ON-DISK bytes under `encoding` and reload the buffer (the
    /// "Reopen with Encoding" action). Reads the file at the panel's `file_path` IN-PROCESS, decodes it
    /// under the chosen encoding ([`Encoding::decode`](super::file_meta::Encoding::decode) — BOM-aware),
    /// and installs the text via [`set_text`](Self::set_text). NO backend call (RISK-005). Records the
    /// new encoding on the doc model. Returns `Ok(())` on success, or `Err(message)` when there is no
    /// file path or the bytes cannot be read — a TYPED outcome the segment surfaces (never a silent
    /// no-op, never a backend rewrite). The undo records as one whole-buffer entry like any reload.
    pub fn reopen_with_encoding(&self, encoding: super::file_meta::Encoding) -> Result<(), String> {
        let path = self.file_path();
        if path.trim().is_empty() {
            return Err(
                "Reopen with Encoding needs a saved file (this buffer is in-memory)".to_owned(),
            );
        }
        let bytes = std::fs::read(&path)
            .map_err(|e| format!("Reopen with Encoding: cannot read {path}: {e}"))?;
        let text = encoding.decode(&bytes);
        *self.encoding.lock().unwrap_or_else(|e| e.into_inner()) = encoding;
        self.set_text(&text);
        Ok(())
    }

    /// MT-071: TEST/host seam — set the active encoding WITHOUT a disk reload (records the encoding the
    /// document was loaded under, e.g. by the MT-010 load path that already decoded the bytes). The
    /// status-bar segment reads it. Distinct from [`reopen_with_encoding`](Self::reopen_with_encoding),
    /// which re-reads + re-decodes the file.
    pub fn set_encoding(&self, encoding: super::file_meta::Encoding) {
        *self.encoding.lock().unwrap_or_else(|e| e.into_inner()) = encoding;
    }

    /// MT-071: the per-document user language override, or `None` while auto-detecting.
    pub fn language_override(&self) -> Option<super::language_mode::LanguageId> {
        self.language_override
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// MT-071: set (or clear with `None`) the per-document user language override — the highest-precedence
    /// detection layer (RISK-003). Persists on the doc model across re-render + re-focus (RISK-004), so the
    /// next [`resolved_language`](Self::resolved_language) reflects it. (Re-highlighting under the new
    /// grammar is a follow-on once more grammars are bundled; the override is recorded + reported now so
    /// the status-bar segment + the resolver honor the user's choice — the contract's "override sticks per
    /// document".)
    pub fn set_language_override(&self, lang: Option<super::language_mode::LanguageId>) {
        *self
            .language_override
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = lang;
    }

    /// MT-071: resolve the document's language with the strict precedence `UserOverride > Shebang >
    /// Content > Extension` (RISK-003), reading the override off the doc model and the shebang/content
    /// off the live buffer. The status-bar language segment shows
    /// [`detected.display_label()`](super::language_mode::LanguageId::display_label) + a source hint.
    pub fn resolved_language(&self) -> super::language_mode::LanguageDetection {
        let override_id = self.language_override();
        // Perf cache (must-fix #4): the status bar resolves the language every frame. Recompute ONLY when
        // the buffer version bumped (an edit) or the override changed; otherwise return the cached
        // detection without a whole-buffer `to_string()` copy. The cache key is `(buffer_version,
        // override)` — both inputs that can change the resolved language.
        let version = self.buffer_version.load(Ordering::Relaxed);
        {
            let cache = self
                .resolved_language_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some((cached_version, cached_override, detection)) = cache.as_ref() {
                if *cached_version == version && *cached_override == override_id {
                    return detection.clone();
                }
            }
        }
        let detection = self.compute_resolved_language(override_id.clone());
        *self
            .resolved_language_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((version, override_id, detection.clone()));
        detection
    }

    /// MT-071: the uncached language resolve (the body [`resolved_language`](Self::resolved_language)
    /// caches). Copies the buffer ONCE per cache miss (an edit / override change), not per frame.
    fn compute_resolved_language(
        &self,
        override_id: Option<super::language_mode::LanguageId>,
    ) -> super::language_mode::LanguageDetection {
        let full_text = self.buffer().to_string();
        // The first line is enough for the shebang sniff; cap the bytes so a huge buffer does not copy
        // its whole head needlessly (the detector only reads the first line of `first_bytes`).
        let first_bytes: Vec<u8> = full_text.as_bytes().iter().take(256).copied().collect();
        // The extension layer needs an extension. Prefer the real file path; for an in-memory buffer
        // (no path) fall back to the panel's grammar `extension` so the extension layer still resolves
        // (a `.rs` in-memory buffer is still Rust). Synthesize a bare `buffer.<ext>` name the
        // extension-of helper reads (no filesystem access).
        let path = self.file_path();
        let synthesized = if path.trim().is_empty() {
            if self.extension.is_empty() {
                String::new()
            } else {
                format!("buffer.{}", self.extension)
            }
        } else {
            path
        };
        let path_opt = if synthesized.is_empty() {
            None
        } else {
            Some(synthesized.as_str())
        };
        super::language_mode::detect_language(
            override_id.as_ref(),
            path_opt,
            &first_bytes,
            &full_text,
        )
    }

    /// MT-071: whether the render-whitespace toggle is on. The MT-001 editor DRAW path reads this to
    /// render middots for spaces + arrows for tabs.
    pub fn render_whitespace(&self) -> bool {
        self.render_whitespace.load(Ordering::Relaxed)
    }

    /// MT-071: set the render-whitespace toggle (the status-bar whitespace segment / an agent flips it).
    /// Keeps the MT-035 3-way mode in lockstep: `true` => All, `false` => None (the boolean segment has no
    /// Boundary state, so it round-trips through the two extremes).
    pub fn set_render_whitespace(&self, on: bool) {
        self.render_whitespace.store(on, Ordering::Relaxed);
        self.render_whitespace_mode
            .store(if on { 2 } else { 0 }, Ordering::Relaxed);
    }

    /// MT-071: flip the render-whitespace toggle and return the NEW value (the segment's left-click).
    pub fn toggle_render_whitespace(&self) -> bool {
        // `fetch_xor(true)` returns the PREVIOUS value, so the new value is its negation.
        let prev = self.render_whitespace.fetch_xor(true, Ordering::Relaxed);
        let now = !prev;
        // Keep the 3-way mode consistent with the boolean flip (All when on, None when off).
        self.render_whitespace_mode
            .store(if now { 2 } else { 0 }, Ordering::Relaxed);
        now
    }

    /// MT-035: the LIVE render-whitespace MODE (None / Boundary / All). The shell threads
    /// `editor_prefs.render_whitespace` in via [`set_render_whitespace_mode`](Self::set_render_whitespace_mode);
    /// `paint_whitespace_glyphs` reads THIS so Boundary and All are no longer collapsed to a single bool.
    pub fn render_whitespace_mode(&self) -> crate::workspace_settings::RenderWhitespaceMode {
        use crate::workspace_settings::RenderWhitespaceMode as M;
        match self.render_whitespace_mode.load(Ordering::Relaxed) {
            1 => M::Boundary,
            2 => M::All,
            _ => M::None,
        }
    }

    /// MT-035: set the LIVE render-whitespace mode (the shell threads the full None/Boundary/All enum from
    /// Settings, fixing the prior Boundary-vs-All lossiness). Keeps the boolean `render_whitespace` in
    /// lockstep (mode != None <=> draw glyphs) so the status-bar toggle + the draw-gate reads stay valid.
    pub fn set_render_whitespace_mode(
        &self,
        mode: crate::workspace_settings::RenderWhitespaceMode,
    ) {
        use crate::workspace_settings::RenderWhitespaceMode as M;
        let code = match mode {
            M::None => 0u8,
            M::Boundary => 1u8,
            M::All => 2u8,
        };
        self.render_whitespace_mode.store(code, Ordering::Relaxed);
        self.render_whitespace
            .store(mode.draws_whitespace(), Ordering::Relaxed);
    }

    /// MT-035: whether the sticky-scroll pinned-header band renders (the shell threads
    /// `editor_prefs.sticky_scroll` in via [`set_sticky_scroll_enabled`](Self::set_sticky_scroll_enabled)).
    pub fn sticky_scroll_enabled(&self) -> bool {
        self.sticky_scroll_enabled.load(Ordering::Relaxed)
    }

    /// MT-035: enable/disable the sticky-scroll band. When `false`, `render_sticky_band` early-returns so
    /// no pinned headers (and no `code_editor_sticky_scroll` nodes) are emitted.
    pub fn set_sticky_scroll_enabled(&self, enabled: bool) {
        self.sticky_scroll_enabled.store(enabled, Ordering::Relaxed);
    }

    /// MT-035: whether the gutter renders line numbers. Reads the EXISTING MT-007 `GutterConfig`
    /// (`show_line_numbers`) so this is the live feature flag the gutter paint path already consumes — no
    /// parallel state.
    pub fn line_numbers_enabled(&self) -> bool {
        self.gutter_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .show_line_numbers
    }

    /// MT-035: enable/disable gutter line numbers (the shell threads `editor_prefs.line_numbers` in from
    /// Settings). Flips the EXISTING `GutterConfig::show_line_numbers` the gutter renderer reads, so the
    /// change takes effect on the running editor's next paint (the gutter also re-measures its width).
    pub fn set_line_numbers_enabled(&self, enabled: bool) {
        self.gutter_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .show_line_numbers = enabled;
    }

    // ── WP-KERNEL-012 wave-6 (S6 item 3): LIVE editor font size + custom syntax palette ─────────────

    /// The LIVE editor font size (pt) — the shell-threaded `editor_font_size`, or the built-in
    /// [`MONO_FONT_SIZE`] default when the shell has not set one. Every panel-body measurement + glyph
    /// paint reads THIS (through [`mono_font`](Self::mono_font)) so the running editor is one consistent
    /// size unit (the MT-054 row-pitch invariant holds at any size: `line_height` is measured from the
    /// SAME font the glyphs paint with).
    pub fn font_size(&self) -> f32 {
        self.font_size
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(MONO_FONT_SIZE)
    }

    /// The monospace `FontId` the editor body measures + paints with, at the LIVE [`font_size`](Self::font_size).
    fn mono_font(&self) -> egui::FontId {
        egui::FontId::monospace(self.font_size())
    }

    /// WP-KERNEL-012 wave-6 (S6 item 3): thread the LIVE editor font size in from the operator's
    /// `editor_prefs.editor_font_size` (the shell calls this from `sync_editor_prefs_to_panel`). Clamped
    /// to the settings range (6..=48 pt). When the size actually changes, the measured-metric caches
    /// (`line_height_px` / `glyph_width_px`) are invalidated so the next frame re-measures at the new size
    /// — that is what resizes the running editor's row height AND glyph advance (no restart). A no-op when
    /// the size is unchanged, so a per-frame sync stays cheap (the caches are not thrashed).
    pub fn set_font_size(&self, size: f32) {
        let clamped = size.clamp(6.0, 48.0);
        // Update the slot and detect a real change WITHOUT holding the font_size lock across the cache
        // locks (keep the lock scopes disjoint).
        let changed = {
            let mut slot = self.font_size.lock().unwrap_or_else(|e| e.into_inner());
            let current = slot.unwrap_or(MONO_FONT_SIZE);
            if (current - clamped).abs() > f32::EPSILON {
                *slot = Some(clamped);
                true
            } else {
                false
            }
        };
        if changed {
            *self
                .line_height_px
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .glyph_width_px
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    /// WP-KERNEL-012 wave-6 (S6 item 3): thread the LIVE syntax palette in from the operator's
    /// `syntax_palette` setting (the shell calls this from `sync_editor_prefs_to_panel`). MT-072 Fix 1:
    /// installing a palette of ANY mode makes [`resolve_highlight_color`](Self::resolve_highlight_color)
    /// route highlight-run colors through the LIVE
    /// [`resolve_scope_color`](crate::code_editor::resolve_scope_color) resolver, so selecting Muted or
    /// Standard, OR editing a Custom swatch, repaints the running editor — the live editor and the Settings
    /// preview swatch AGREE for every mode. Theme tokens remain the fallback only when no palette is set.
    pub fn set_syntax_palette(&self, palette: crate::workspace_settings::SyntaxPalette) {
        *self
            .syntax_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(palette);
    }

    /// Resolve the color a highlight `scope` should paint with, honoring a LIVE syntax palette of ANY mode
    /// (Muted / Standard / Custom) when one is installed (MT-072 Fix 1), and otherwise the theme's
    /// [`scope_to_color`]. The panel body draw paths call this for every highlighted run, so a palette-mode
    /// switch or a Custom swatch edit changes the painted color in the SAME frame — the running editor and
    /// the Settings preview swatch AGREE for every mode. `pub` so a kittest can assert the render-path color
    /// directly.
    pub fn resolve_highlight_color(
        &self,
        scope: HighlightScope,
        syntax: &HsSyntaxTokens,
    ) -> egui::Color32 {
        let palette = self
            .syntax_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match palette.as_ref() {
            // MT-072 Fix 1: a LIVE palette of ANY mode resolves through the SAME `resolve_scope_color` the
            // Settings preview swatch uses (Muted/Standard tables; Custom overrides with a Standard fallback
            // for un-overridden scopes), so selecting Muted or Standard recolors the RUNNING editor — not
            // only the preview. Theme tokens are the fallback ONLY when no palette selection is installed.
            Some(p) => crate::code_editor::resolve_scope_color(scope, p),
            None => scope_to_color(scope, syntax),
        }
    }

    // ── WP-KERNEL-012 MT-035 wave-7: LIVE line-height multiplier + bracket-match/indent-guide gating ──

    /// The LIVE row-height multiplier — the shell-threaded `editor_prefs.line_height`, or `1.0`
    /// (single-spaced) when the shell has not set one. [`line_height`](Self::line_height) multiplies the
    /// measured mono-font row height by this so lines are spaced by the multiplier.
    pub fn line_height_multiplier(&self) -> f32 {
        self.line_height_multiplier
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(1.0)
    }

    /// WP-KERNEL-012 MT-035 wave-7: thread the LIVE row-height multiplier in from the operator's
    /// `editor_prefs.line_height` (the shell calls this from `sync_editor_prefs_to_panel`). Clamped to the
    /// settings range (1.0..=2.0). When the multiplier actually changes, the measured row-height cache
    /// (`line_height_px`) is invalidated so the next frame re-measures and re-scales — that is what respaces
    /// the running editor's rows (no restart). A no-op when unchanged, so a per-frame sync stays cheap.
    pub fn set_line_height(&self, multiplier: f32) {
        let clamped = multiplier.clamp(
            *crate::workspace_settings::EDITOR_LINE_HEIGHT_RANGE.start(),
            *crate::workspace_settings::EDITOR_LINE_HEIGHT_RANGE.end(),
        );
        let changed = {
            let mut slot = self
                .line_height_multiplier
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let current = slot.unwrap_or(1.0);
            if (current - clamped).abs() > f32::EPSILON {
                *slot = Some(clamped);
                true
            } else {
                false
            }
        };
        if changed {
            *self
                .line_height_px
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    /// WP-KERNEL-012 MT-035 wave-7: whether the matching-bracket highlight renders (the shell threads
    /// `editor_prefs.bracket_matching` in via [`set_bracket_matching_enabled`](Self::set_bracket_matching_enabled)).
    pub fn bracket_matching_enabled(&self) -> bool {
        self.bracket_matching_enabled.load(Ordering::Relaxed)
    }

    /// WP-KERNEL-012 MT-035 wave-7: enable/disable the matching-bracket highlight. When `false`,
    /// [`matching_bracket_at`](Self::matching_bracket_at) returns `None` and `paint_chrome_decorations`
    /// paints no matched-bracket box.
    pub fn set_bracket_matching_enabled(&self, enabled: bool) {
        self.bracket_matching_enabled
            .store(enabled, Ordering::Relaxed);
    }

    /// WP-KERNEL-012 MT-035 wave-7: whether vertical indent-guide lines render (the shell threads
    /// `editor_prefs.indent_guides` in via [`set_indent_guides_enabled`](Self::set_indent_guides_enabled)).
    pub fn indent_guides_enabled(&self) -> bool {
        self.indent_guides_enabled.load(Ordering::Relaxed)
    }

    /// WP-KERNEL-012 MT-035 wave-7: enable/disable the indent-guide lines. When `false`,
    /// `paint_chrome_decorations` skips the guide pass and
    /// [`indent_guide_count_for_line`](Self::indent_guide_count_for_line) reports `0`.
    pub fn set_indent_guides_enabled(&self, enabled: bool) {
        self.indent_guides_enabled.store(enabled, Ordering::Relaxed);
    }

    /// The gated matching-bracket computation shared by the render path (which passes its already-held
    /// buffer lock) and the public [`matching_bracket_at`](Self::matching_bracket_at) accessor. Returns the
    /// `(open_byte, close_byte)` of the pair the caret at `cursor_byte` is on/next to, or `None` when the
    /// toggle is OFF or no bracket is adjacent. One logic path so the render and the test never drift.
    fn matching_bracket_pair(
        &self,
        buffer: &TextBuffer,
        cursor_byte: usize,
    ) -> Option<(usize, usize)> {
        if !self.bracket_matching_enabled() {
            return None;
        }
        find_matching_bracket(buffer, cursor_byte).map(
            |BracketMatch {
                 open_byte,
                 close_byte,
             }| (open_byte, close_byte),
        )
    }

    /// WP-KERNEL-012 MT-035 wave-7: the `(open_byte, close_byte)` of the bracket pair the caret at
    /// `cursor_byte` is on/next to (VS Code adjacency), gated by the bracket-matching toggle — `None` when
    /// the toggle is OFF or no bracket is adjacent. This is the SAME gated computation the
    /// `paint_chrome_decorations` matched-bracket box uses, exposed so a test can prove the toggle drives
    /// the mounted panel BOTH directions without manipulating cursor state.
    pub fn matching_bracket_at(&self, cursor_byte: usize) -> Option<(usize, usize)> {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.matching_bracket_pair(&buffer, cursor_byte)
    }

    /// WP-KERNEL-012 MT-035 wave-7: the number of vertical indent guides that WOULD be drawn for
    /// `buffer_line` (equal to the line's indent level — `paint_chrome_decorations` draws one guide per
    /// level `1..=level`), gated by the indent-guides toggle — `0` when the toggle is OFF. Exposed so a
    /// test can prove the toggle drives the mounted panel BOTH directions.
    pub fn indent_guide_count_for_line(&self, buffer_line: usize) -> usize {
        if !self.indent_guides_enabled() {
            return 0;
        }
        let (tab_width, _) = self.indent_settings();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        indent_level_of(&buffer, buffer_line, tab_width.max(1))
    }

    // ── MT-054 word wrap (Alt+Z) — toggle + state ─────────────────────────────────────────────────

    /// MT-054: the current word-wrap configuration (for tests / the host / the AccessKit node value).
    pub fn wrap_config(&self) -> WrapConfig {
        *self.wrap_config.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// MT-054: whether word wrap is currently enabled (Alt+Z toggles it). Persisted on the panel state.
    pub fn is_wrap_enabled(&self) -> bool {
        self.wrap_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enabled
    }

    /// MT-054: flip word wrap on/off and return the NEW enabled state. The single mutation point both
    /// the Alt+Z shortcut and the `editor-wrap-toggle` AccessKit node route through, so the toggle is
    /// deterministic for a swarm agent and persisted on the panel state (AC-005). Render/decoration only
    /// — NO buffer mutation (AC-007).
    pub fn toggle_wrap(&self) -> bool {
        let mut cfg = self.wrap_config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.enabled = !cfg.enabled;
        // MT-072 Fix 3: mark this as a USER-initiated toggle (Alt+Z / the "Wrap" button / the
        // editor-wrap-toggle node) so the host writes it back into the persisted editor prefs (Alt+Z
        // persistence). A prefs->panel push via `set_wrap_enabled` does NOT set this flag, so the write-back
        // never ping-pongs.
        self.wrap_toggled_by_user.store(true, Ordering::Relaxed);
        cfg.enabled
    }

    /// MT-072 Fix 3 (MT-054 wrap-persistence closeout): take a pending USER-initiated wrap toggle (set by
    /// [`toggle_wrap`](Self::toggle_wrap), the single mutation point Alt+Z / the visible "Wrap" button / the
    /// `editor-wrap-toggle` AccessKit node all route through). Returns `Some(is_wrap_enabled)` exactly once
    /// per user toggle and clears the flag, so the host writes the change back into the persisted
    /// `editor_prefs.word_wrap`; returns `None` when no user toggle is pending. A prefs->panel
    /// [`set_wrap_enabled`](Self::set_wrap_enabled) push does NOT set the flag, so it never round-trips back.
    pub fn take_user_wrap_toggle(&self) -> Option<bool> {
        if self.wrap_toggled_by_user.swap(false, Ordering::Relaxed) {
            Some(self.is_wrap_enabled())
        } else {
            None
        }
    }

    /// MT-054: explicitly set the wrap-enabled state (host / settings / a test). Persisted on the panel.
    pub fn set_wrap_enabled(&self, enabled: bool) {
        self.wrap_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .enabled = enabled;
    }

    /// MT-054: set a fixed wrap COLUMN (`wordWrapColumn`), or `None` to wrap at the viewport edge
    /// (`wordWrap: on`). The host plumbs this from the editor-settings layer; tests use it to force a
    /// deterministic wrap width without a real viewport.
    pub fn set_wrap_column(&self, wrap_column: Option<usize>) {
        self.wrap_config
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .wrap_column = wrap_column;
    }

    /// MT-054: the stable AccessKit author_id for this panel's word-wrap toggle node, with the instance
    /// suffix when present (RISK-004). The default single panel uses the bare `editor-wrap-toggle` id the
    /// MT names so a swarm agent matches it exactly.
    pub fn wrap_toggle_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_WRAP_TOGGLE_AUTHOR_ID)
    }

    /// MT-054: the fixed `egui::Id` for the word-wrap toggle node (band slot 290 for the default panel;
    /// hashed for instances). See [`container_id`](Self::container_id) for the safety rationale.
    fn wrap_toggle_node_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(EDITOR_WRAP_TOGGLE_NODE_ID) }
        } else {
            egui::Id::new(self.wrap_toggle_author_id())
        }
    }

    /// MT-054: dispatch the `editor-wrap-toggle` AccessKit action by author_id (the swarm-agent path).
    /// Returns the NEW enabled state when the id matched this panel's toggle (so a test/agent can read
    /// the result), or `None` for an unmatched id (a benign no-op, never a panic — RISK guard).
    pub fn toggle_wrap_by_author_id(&self, author_id: &str) -> Option<bool> {
        if author_id == self.wrap_toggle_author_id() {
            Some(self.toggle_wrap())
        } else {
            None
        }
    }

    /// MT-051: build the [`LineEditContext`] for one dispatch batch from the panel's language-family id +
    /// the operator's tab settings (the "build the context once per dispatch batch" rule). The language id
    /// is the SAME stable family id the highlighter carries (RISK-007 — no second language enum).
    fn line_edit_context(&self) -> line_ops::LineEditContext {
        let (tab_size, insert_spaces) = self.indent_settings();
        line_ops::LineEditContext::new(self.language_id, tab_size, insert_spaces)
    }

    /// MT-051: run a `line_ops` transform with single-undo coalescing (AC-007 / RISK-003). Snapshots the
    /// whole buffer BEFORE, runs `transform` (which mutates the buffer + cursor set in place), and — iff
    /// the buffer text actually changed — snapshots AFTER and queues ONE `(description, before, after)`
    /// undo entry (drained by the factory render into `interop_adapter::push_code_edit_undo`, the SAME bus
    /// boundary every code edit's undo is recorded at) and refreshes the highlight cache. No parallel undo
    /// stack is created. Returns whether the buffer changed.
    fn apply_line_transform(
        &self,
        description: &'static str,
        transform: impl FnOnce(&mut TextBuffer, &mut CursorSet, &line_ops::LineEditContext) -> bool,
    ) -> bool {
        let ctx = self.line_edit_context();
        // Snapshot BEFORE (ropey clone is O(1) — the MT-035 single-undo pattern).
        let before = self.with_buffer(|b| b.to_string());
        let changed = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let mut set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            transform(&mut buffer, &mut set, &ctx)
        };
        if !changed {
            return false;
        }
        let after = self.with_buffer(|b| b.to_string());
        if after == before {
            // The transform reported a change but the text is identical (defensive): nothing to undo.
            return false;
        }
        *self
            .pending_line_op_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            Some((description, before.clone(), after.clone()));
        self.record_code_edit_mutation_text(&before, &after);
        // A line transform replaces whole rows (move/delete/duplicate/comment) WITHOUT feeding tree-sitter
        // an `InputEdit`, so the highlighter's cached incremental tree would describe byte offsets past the
        // new buffer and panic on re-highlight. Reset the highlighter to a clean FULL parse before
        // refreshing the spans (the format/undo `set_text` path replaces the whole buffer too; line
        // transforms are the in-place sibling that needs the same incremental-state reset).
        self.reset_highlighter();
        self.refresh();
        true
    }

    /// MT-051: rebuild the tree-sitter highlighter for this document's grammar from scratch, discarding the
    /// cached incremental parse tree. Called after a structural line transform so the next
    /// [`ensure_highlight_cache`](Self::ensure_highlight_cache) does a clean FULL parse of the new buffer
    /// (RISK-002 — never an incremental re-parse against a tree whose node offsets exceed the new, possibly
    /// shorter, buffer). A no-language / unregistered-extension document keeps its `None` highlighter
    /// (plain text, no highlighting). Cheap: the grammar load is a pointer copy + a query compile, done only
    /// on an explicit edit, never per frame.
    fn reset_highlighter(&self) {
        let fresh =
            LanguageRegistry::with_bundled_languages().highlighter_for_extension(&self.extension);
        *self.highlighter.lock().unwrap_or_else(|e| e.into_inner()) = fresh;
    }

    /// MT-051: take the queued line-transform undo snapshot `(description, before, after)` the factory
    /// render pushes onto the shared unified-undo bus as ONE entry. `None` when no transform applied since
    /// the last drain.
    pub fn take_pending_line_op_undo(&self) -> Option<(&'static str, String, String)> {
        self.pending_line_op_undo
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// MT-049: the screen position to anchor the quick-fix lightbulb / menu for `line` — the start of the
    /// line in the gutter strip's lightbulb column (next to the MT-007 diagnostic glyphs). `None` before
    /// the first frame / when the line is off-screen.
    fn quickfix_line_screen_pos(&self, line: usize) -> Option<egui::Pos2> {
        let glyph_width = (*self
            .glyph_width_px
            .lock()
            .unwrap_or_else(|e| e.into_inner()))
        .unwrap_or(8.0);
        self.screen_pos_for_line_col(line, 0, glyph_width)
    }

    /// Build completion items from backend code-nav symbol projections (the React `suggestions.map`).
    /// The deterministic mapping the off-thread completion task + tests use.
    pub fn completions_from_symbols(symbols: &[CodeSymbolNavProjection]) -> Vec<CompletionItem> {
        symbols.iter().map(CompletionItem::from_symbol).collect()
    }

    /// Push warning gutter markers for every NOT-FRESH symbol projection (AC-007): the native port of
    /// `refreshHandshakeCodeIntelligenceMarkers`'s staleness branch. Each stale symbol with a definition
    /// span yields a Warning marker on its line. Replaces the current diagnostic markers via the MT-007
    /// [`push_diagnostics`] slot (so a swarm agent / a screenshot sees the staleness dot in the gutter).
    /// Returns the number of markers pushed. A diagnostics push does NOT bump `buffer_version` (the
    /// MT-007 perf invariant).
    pub fn push_staleness_markers(&self, symbols: &[CodeSymbolNavProjection]) -> usize {
        let markers: Vec<GutterMarker> = symbols.iter().filter_map(staleness_marker_for).collect();
        let count = markers.len();
        self.push_diagnostics(markers);
        count
    }

    /// Drain the LSP `publishDiagnostics` channel and map any pending notification onto the gutter via
    /// [`push_diagnostics`] (AC-008). Called each frame (cheap when empty). Only the diagnostics whose
    /// URI matches this panel's file are applied; the editor maps `range.start.line` (0-based) to a gutter
    /// line and the LSP severity to a [`DiagnosticSeverity`]. Returns the number of markers pushed if a
    /// notification was drained, else `None` (no notification this frame — leave the markers as-is).
    pub fn drain_lsp_diagnostics(&self) -> Option<usize> {
        let expected_uri = self.format_uri()?;
        let needs_subscription = self
            .lsp_diagnostics_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_none();
        if needs_subscription {
            let receiver = {
                // Every panel gets its own broadcast cursor, including panels that share one client.
                let client = self.lsp_client.lock().unwrap_or_else(|e| e.into_inner());
                client.subscribe_diagnostics()
            };
            *self
                .lsp_diagnostics_rx
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(receiver);
        }
        let live_buffer_version = self.buffer_version.load(Ordering::Acquire) as i64;
        let prior_version = self
            .lsp_diagnostics_version
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .filter(|(uri, _)| same_lsp_document_uri(&expected_uri, uri))
            .map(|(_, version)| *version);
        let mut latest: Option<PublishedDiagnostics> = None;
        if let Some(rx) = self
            .lsp_diagnostics_rx
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            loop {
                let published = match rx.try_recv() {
                    Ok(published) => published,
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::TryRecvError::Empty)
                    | Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
                };
                if !same_lsp_document_uri(&expected_uri, &published.uri) {
                    continue;
                }
                if let Some(version) = published.version {
                    let newest_known = latest
                        .as_ref()
                        .and_then(|candidate| candidate.version)
                        .into_iter()
                        .chain(prior_version)
                        .chain(std::iter::once(live_buffer_version))
                        .max()
                        .unwrap_or(live_buffer_version);
                    if version < newest_known {
                        continue;
                    }
                }
                latest = Some(published);
            }
        }
        let published = latest?;
        if let Some(version) = published.version {
            *self
                .lsp_diagnostics_version
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some((expected_uri, version));
        }
        let markers = lsp_diagnostics_to_markers(&published);
        let count = markers.len();
        self.push_diagnostics(markers);
        Some(count)
    }

    /// Mark a buffer edit happened now (the completion-debounce clock — implementation note 2).
    pub fn mark_edit_now(&self) {
        *self
            .last_edit_instant
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(std::time::Instant::now());
    }

    /// True when the completion debounce window ([`COMPLETION_DEBOUNCE_MS`]) has elapsed since the last
    /// edit (or no edit has happened) — i.e. it is safe to fire a completion request (RISK-002).
    pub fn completion_debounce_elapsed(&self) -> bool {
        match *self
            .last_edit_instant
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        {
            Some(at) => at.elapsed() >= std::time::Duration::from_millis(COMPLETION_DEBOUNCE_MS),
            None => true,
        }
    }

    /// Spawn an off-thread completion request for `prefix`, asking the configured/running LSP first and
    /// using the Handshake code-nav lookup only when LSP is absent, unavailable, or returns no items.
    /// Every delivery carries its request generation + buffer/caret/document/workspace identity; the UI
    /// drain rejects stale responses before they can replace a newer popup. `runtime` is the app's tokio
    /// handle (the egui thread never blocks — HBR-QUIET).
    pub fn trigger_completion(&self, runtime: &tokio::runtime::Handle, prefix: &str) {
        self.close_completion();
        let generation = self.completion_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let workspace_id = self.workspace_id();
        let lsp = self.lsp_client();
        let document_uri = self.lsp_uri();
        let lsp_available = (lsp.is_configured() || lsp.is_running()) && document_uri.is_some();
        // Empty/one-character prefixes are valid LSP completion requests. The two-character floor is
        // solely a CodeNav fallback load guard.
        let code_nav_eligible = !workspace_id.is_empty() && prefix.chars().count() >= 2;
        if !lsp_available && !code_nav_eligible {
            return;
        }
        let cursor_byte = self.primary_cursor_offset();
        let request = CodeIntelligenceRequestIdentity {
            generation,
            buffer_version: self.buffer_version.load(Ordering::Relaxed),
            cursor_byte,
            document_uri,
            workspace_id: workspace_id.clone(),
            query: prefix.to_owned(),
        };
        let anchor = self
            .cursor_screen_pos()
            .unwrap_or_else(|| egui::pos2(40.0, 40.0));
        // Capture a possible fallback cache hit now. It is deliberately NOT delivered before the LSP
        // request: configured LSP is the primary authority even when CodeNav has cached data.
        let cached_fallback = code_nav_eligible
            .then(|| {
                self.code_nav_cache
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .get(&workspace_id, prefix)
            })
            .flatten();
        let code_nav = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let cell = Arc::clone(&self.completion_result);
        let position = self.lsp_position_at(cursor_byte);
        runtime.spawn(async move {
            let lsp_items = if lsp_available {
                lsp.completion_after_sync(
                    request.document_uri.as_deref().unwrap_or_default(),
                    position,
                )
                .await
            } else {
                Vec::new()
            };
            let (items, code_nav_batch) = if lsp_items.is_empty() {
                let symbols = if let Some(cached) = cached_fallback {
                    cached
                } else if !code_nav_eligible {
                    Vec::new()
                } else {
                    code_nav
                        .lookup_symbols(&request.workspace_id, &request.query, SYMBOL_LOOKUP_LIMIT)
                        .await
                        .unwrap_or_default()
                };
                let items = symbols.iter().map(CompletionItem::from_symbol).collect();
                let batch = code_nav_eligible.then(|| (request.query.clone(), symbols));
                (items, batch)
            } else {
                (
                    lsp_items.iter().map(Self::completion_from_lsp).collect(),
                    None,
                )
            };
            let delivery = CompletionDelivery {
                request,
                anchor,
                items,
                code_nav_batch,
            };
            if let Ok(mut slot) = cell.lock() {
                let replace = slot
                    .as_ref()
                    .map(|current| current.request.generation <= delivery.request.generation)
                    .unwrap_or(true);
                if replace {
                    *slot = Some(delivery);
                }
            }
        });
    }

    /// Map the LSP completion vocabulary into the popup's shared item vocabulary.
    fn completion_from_lsp(item: &LspCompletionItem) -> CompletionItem {
        let kind = match item.kind {
            Some(7 | 8 | 22) => super::code_nav::CompletionKind::Class,
            Some(13 | 20) => super::code_nav::CompletionKind::Enum,
            Some(5 | 10) => super::code_nav::CompletionKind::Field,
            Some(9 | 17 | 19) => super::code_nav::CompletionKind::Module,
            Some(6 | 11 | 12 | 21) => super::code_nav::CompletionKind::Variable,
            _ => super::code_nav::CompletionKind::Function,
        };
        let detail = item.detail.clone().unwrap_or_else(|| "LSP".to_owned());
        CompletionItem {
            label: item.label.clone(),
            insert_text: item.insert_text.clone(),
            kind,
            detail: detail.clone(),
            documentation: detail,
            symbol_entity_id: String::new(),
        }
    }

    /// Update the hover-dwell tracker for the current cursor byte offset and return `true` once per
    /// settled offset after the cursor has rested at the SAME offset for at least [`HOVER_DWELL_MS`]
    /// (implementation note 3). A cursor move resets the dwell. The editor calls this each frame with the
    /// live cursor offset; on a `true` it calls [`trigger_hover`](Self::trigger_hover) to fetch the hover.
    pub fn update_hover_dwell(&self, cursor_byte_offset: usize) -> bool {
        let mut guard = self.hover_dwell.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_mut() {
            Some((offset, since, fired)) if *offset == cursor_byte_offset => {
                if *fired {
                    return false;
                }
                if since.elapsed() >= std::time::Duration::from_millis(HOVER_DWELL_MS) {
                    *fired = true;
                    true
                } else {
                    false
                }
            }
            _ => {
                // New offset (or first dwell): restart the dwell clock.
                *guard = Some((cursor_byte_offset, std::time::Instant::now(), false));
                false
            }
        }
    }

    /// Spawn an off-thread hover request for `word`, asking LSP first and falling back to Handshake
    /// code-nav only when LSP is absent, unavailable, or empty. The generation + live-state identity is
    /// validated on delivery so an older hover cannot replace a newer caret/document result.
    pub fn trigger_hover(&self, runtime: &tokio::runtime::Handle, word: &str) {
        self.close_hover();
        let generation = self.hover_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let workspace_id = self.workspace_id();
        if word.trim().is_empty() {
            return;
        }
        let lsp = self.lsp_client();
        let document_uri = self.lsp_uri();
        let lsp_available = (lsp.is_configured() || lsp.is_running()) && document_uri.is_some();
        if !lsp_available && workspace_id.is_empty() {
            return;
        }
        let cursor_byte = self.primary_cursor_offset();
        let request = CodeIntelligenceRequestIdentity {
            generation,
            buffer_version: self.buffer_version.load(Ordering::Relaxed),
            cursor_byte,
            document_uri,
            workspace_id: workspace_id.clone(),
            query: word.to_owned(),
        };
        let anchor = self
            .cursor_screen_pos()
            .unwrap_or_else(|| egui::pos2(40.0, 40.0));
        let code_nav = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let cell = Arc::clone(&self.hover_result);
        let position = self.lsp_position_at(cursor_byte);
        let current_file_path = self.file_path();
        runtime.spawn(async move {
            if lsp_available {
                if let Some(lsp_hover) = lsp
                    .hover_after_sync(
                        request.document_uri.as_deref().unwrap_or_default(),
                        position,
                    )
                    .await
                {
                    let base_hover = HoverState {
                        markdown: lsp_hover.value,
                        display_name: request.query.clone(),
                        anchor,
                        definition_target: None,
                    };
                    let delivery = HoverDelivery {
                        hover: Some(base_hover.clone()),
                        request: request.clone(),
                        code_nav_batch: None,
                    };
                    if let Ok(mut slot) = cell.lock() {
                        let replace = slot
                            .as_ref()
                            .map(|current| {
                                current.request.generation <= delivery.request.generation
                            })
                            .unwrap_or(true);
                        if replace {
                            *slot = Some(delivery);
                        }
                    }
                    // Preserve the hover immediately, then enrich it with the lossless definition target
                    // when the server resolves one. A slow definition request never withholds hover text.
                    if let Some(location) = lsp
                        .goto_definition_after_sync(
                            request.document_uri.as_deref().unwrap_or_default(),
                            position,
                        )
                        .await
                    {
                        let mut linked_hover = base_hover;
                        linked_hover.definition_target =
                            Some(navigation_location_from_lsp(location));
                        let linked = HoverDelivery {
                            hover: Some(linked_hover),
                            request,
                            code_nav_batch: None,
                        };
                        if let Ok(mut slot) = cell.lock() {
                            let replace = slot
                                .as_ref()
                                .map(|current| {
                                    current.request.generation <= linked.request.generation
                                })
                                .unwrap_or(true);
                            if replace {
                                *slot = Some(linked);
                            }
                        }
                    }
                    return;
                }
            }
            if request.workspace_id.is_empty() {
                let delivery = HoverDelivery {
                    request,
                    hover: None,
                    code_nav_batch: None,
                };
                if let Ok(mut slot) = cell.lock() {
                    let replace = slot
                        .as_ref()
                        .map(|current| current.request.generation <= delivery.request.generation)
                        .unwrap_or(true);
                    if replace {
                        *slot = Some(delivery);
                    }
                }
                return;
            }
            // Prefix results are backend-key ordered; bind the exact identifier before a sibling such
            // as `address` so hover content and its definition link describe the word under the caret.
            let symbols = code_nav
                .lookup_symbols(&request.workspace_id, &request.query, 5)
                .await
                .unwrap_or_default();
            let code_nav_batch = Some((request.query.clone(), symbols.clone()));
            let Some(lookup_symbol) = preferred_symbol_for_identifier_in_file(
                symbols,
                &request.query,
                &current_file_path,
            ) else {
                let delivery = HoverDelivery {
                    request,
                    hover: None,
                    code_nav_batch,
                };
                if let Ok(mut slot) = cell.lock() {
                    let replace = slot
                        .as_ref()
                        .map(|current| current.request.generation <= delivery.request.generation)
                        .unwrap_or(true);
                    if replace {
                        *slot = Some(delivery);
                    }
                }
                return;
            };
            let symbol = if lookup_symbol.symbol_entity_id.is_empty() {
                lookup_symbol
            } else {
                match code_nav.get_symbol(&lookup_symbol.symbol_entity_id).await {
                    Ok(resp)
                        if !resp.symbol.symbol_entity_id.is_empty()
                            || !resp.symbol.display_name.is_empty() =>
                    {
                        resp.symbol
                    }
                    _ => lookup_symbol,
                }
            };
            let lens_doc = if let (Some(path), Some(staleness)) = (
                symbol_file_path(&symbol.symbol_key),
                symbol.staleness.as_ref(),
            ) {
                match (
                    staleness.indexed_content_hash.as_deref(),
                    staleness.indexed_parser_version.as_deref(),
                ) {
                    (Some(hash), Some(parser_version)) => code_nav
                        .get_file_lens(&workspace_id, &path, hash, parser_version)
                        .await
                        .ok()
                        .and_then(|lens| {
                            lens.entries
                                .iter()
                                .find(|entry| entry.symbol_entity_id == symbol.symbol_entity_id)
                                .and_then(|entry| entry.doc.clone())
                        }),
                    _ => None,
                }
            } else {
                None
            };
            let definition_target = code_nav_location_from_symbol(&symbol, &current_file_path);
            let markdown = super::code_nav::markdown_for_symbol(&symbol, lens_doc.as_deref());
            let delivery = HoverDelivery {
                request,
                hover: Some(HoverState {
                    markdown,
                    display_name: symbol.display_name.clone(),
                    anchor,
                    definition_target,
                }),
                code_nav_batch,
            };
            if let Ok(mut slot) = cell.lock() {
                let replace = slot
                    .as_ref()
                    .map(|current| current.request.generation <= delivery.request.generation)
                    .unwrap_or(true);
                if replace {
                    *slot = Some(delivery);
                }
            }
        });
    }

    /// The screen position of the CENTER of the gutter row that paints buffer `line` on the most recent
    /// frame, or `None` if that line was not painted (off-screen) / no frame has rendered. The
    /// deterministic basis for the AC-005 gutter-click test (compute the exact pixel to click for a
    /// known line). Targets the breakpoint sub-column (left of the gutter) so the click lands on the
    /// breakpoint area, not the line-number or diagnostic column.
    pub fn gutter_breakpoint_pos_for_line(&self, line: usize) -> Option<egui::Pos2> {
        let rows = self
            .last_gutter_paint_rows
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let geometry = (*self
            .last_gutter_geometry
            .lock()
            .unwrap_or_else(|e| e.into_inner()))?;
        let rect = (*self
            .last_gutter_rect
            .lock()
            .unwrap_or_else(|e| e.into_inner()))?;
        let row_idx = rows
            .iter()
            .position(|row| row.line == line && row.is_first_fragment)?;
        let y =
            geometry.origin.y + row_idx as f32 * geometry.line_height + geometry.line_height * 0.5;
        // Click in the breakpoint sub-column (a little right of the strip's left edge).
        let x = rect.left() + 12.0;
        Some(egui::pos2(x, y))
    }

    /// The screen position of the CENTER of the FOLD sub-column for buffer `line` on the most recent
    /// frame (the fold triangle is left-of-number; this returns its center x), or `None` if the line was
    /// not painted. The basis for the AC-006 gutter fold-click test.
    pub fn gutter_fold_pos_for_line(&self, line: usize) -> Option<egui::Pos2> {
        let rows = self
            .last_gutter_paint_rows
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let geometry = (*self
            .last_gutter_geometry
            .lock()
            .unwrap_or_else(|e| e.into_inner()))?;
        let rect = (*self
            .last_gutter_rect
            .lock()
            .unwrap_or_else(|e| e.into_inner()))?;
        let config = *self.gutter_config.lock().unwrap_or_else(|e| e.into_inner());
        let row_idx = rows
            .iter()
            .position(|row| row.line == line && row.is_first_fragment)?;
        let y =
            geometry.origin.y + row_idx as f32 * geometry.line_height + geometry.line_height * 0.5;
        // The fold column sits after the breakpoint column. Mirror `gutter::Gutter::render`'s anchors.
        let breakpoint_w = if config.show_breakpoints { 16.0 } else { 0.0 };
        let fold_w = crate::code_editor::gutter::fold_column_width(geometry.char_width);
        let x = rect.left() + 4.0 + breakpoint_w + fold_w * 0.5;
        Some(egui::pos2(x, y))
    }

    /// The stable AccessKit author_id for this panel's minimap, with the instance suffix when present.
    pub fn minimap_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_MINIMAP_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for this panel's outline tree, with the instance suffix.
    pub fn outline_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_OUTLINE_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for this panel's go-to-line input, with the instance suffix.
    pub fn goto_line_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_GOTO_LINE_AUTHOR_ID)
    }

    /// The screen rect the minimap occupied on the most recent frame, or `None` before the first render
    /// / while the minimap is hidden. The deterministic basis for the AC-006 midpoint-click test (which
    /// computes the exact pixel to click) + the AC-003 three-panel layout test (which asserts the
    /// minimap's right placement + ~80px width).
    pub fn last_minimap_rect(&self) -> Option<egui::Rect> {
        *self
            .last_minimap_rect
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// The screen rect the outline panel occupied on the most recent frame, or `None` before the first
    /// render / while it is hidden. The basis for the AC-003 three-panel layout test (left placement +
    /// width vs the minimap).
    pub fn last_outline_rect(&self) -> Option<egui::Rect> {
        *self
            .last_outline_rect
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// The cached minimap per-row colors for this frame, recomputing the O(spans) pass ONLY on a cache
    /// miss — a buffer edit (`version` moved), a panel resize (`painted_rows` changed), a theme flip
    /// (`dark_mode` changed), or a Custom syntax-palette edit. On a hit (the common per-frame case) this
    /// is a cheap key compare + clone of the small `Vec<Color32>` (at most a few hundred rows), so the
    /// minimap render stays O(painted_rows) instead of O(spans) — the MT-002 frame-budget protection on a
    /// 100k-line file.
    fn minimap_row_colors(
        &self,
        painted_rows: usize,
        ratio: usize,
        dark_mode: bool,
        version: u64,
    ) -> Vec<egui::Color32> {
        let syntax_palette = self
            .syntax_palette
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let key = (version, painted_rows, dark_mode);
        {
            let cache = self
                .minimap_row_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some((colors, v, rows, dm, palette)) = cache.as_ref() {
                if (*v, *rows, *dm) == key
                    && palette == &syntax_palette
                    && colors.len() == painted_rows
                {
                    return colors.clone(); // cache hit: no span fetch / no O(spans) re-walk this frame.
                }
            }
        }
        // Miss (edit / resize / theme flip): fetch the cached highlight spans (no extra parse — the
        // highlight cache is already current) and run the single O(spans) color pass, then cache it.
        let colors = {
            let span_cache = self
                .highlight_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let empty: Vec<HighlightSpan> = Vec::new();
            let spans = span_cache
                .as_ref()
                .map(|(s, _)| s.spans.as_slice())
                .unwrap_or(&empty);
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            Minimap::compute_row_colors(
                &buffer,
                spans,
                painted_rows,
                ratio,
                dark_mode,
                syntax_palette.as_ref(),
            )
        };
        *self
            .minimap_row_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((
            colors.clone(),
            version,
            painted_rows,
            dark_mode,
            syntax_palette,
        ));
        colors
    }

    /// The per-frame virtualization diagnostics from the most recent `show` (MT-002 step 4). Before
    /// the first render `frame_lines_rendered` is 0; `buffer_len_lines` is always the document size.
    pub fn perf_stats(&self) -> PerfStats {
        *self.perf.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The measured row height used by the live virtualized painter. Exposed as a diagnostic seam so
    /// performance proofs can derive a viewport-relative paint cap instead of relying on a magic row
    /// count. `None` until the first frame has measured the configured monospace font.
    pub fn measured_line_height_px(&self) -> Option<f32> {
        *self
            .line_height_px
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// The line index range painted on the most recent `show` — the exact `row_range`
    /// `egui::ScrollArea::show_rows` selected (AC-007; egui applies no overscan). `0..0` before the
    /// first render. Lets a test/agent assert exactly which lines are on screen — the deterministic
    /// basis for AC-003 ("line 0 not painted; the scrolled-to region is") and the overlay-positioning
    /// seam MT-003+ reads.
    pub fn last_visible_range(&self) -> std::ops::Range<usize> {
        self.last_visible_range
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Current painted viewport in canonical buffer-line coordinates. Cross-surface navigation
    /// completion uses this instead of the fold-compressed visible-row range so a hidden definition
    /// cannot be reported as visibly revealed.
    pub fn last_visible_buffer_range(&self) -> std::ops::Range<usize> {
        self.last_painted_buffer_range(self.buffer().len_lines())
    }

    /// Whether this exact canonical buffer line was painted in the most recent frame. Unlike a
    /// bounding range check, this remains false for lines hidden inside a fold and works under wrap.
    pub fn is_buffer_line_painted(&self, line: usize) -> bool {
        self.last_gutter_paint_rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .iter()
            .any(|row| row.line == line)
    }

    /// The live vertical scroll offset in pixels from egui's own `ScrollArea` state, including
    /// fractional partial-row offsets. `0.0` before the first render.
    pub fn last_scroll_offset_px(&self) -> f32 {
        *self
            .last_scroll_offset_px
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Request that the next `show` scrolls the viewport to `offset_px` (pixels from the content top).
    /// One-shot: the request is consumed (and cleared) on the next frame so the user can scroll freely
    /// afterward. The seam later MTs' go-to-line / scroll-to-symbol actions build on.
    pub fn scroll_to_offset_px(&self, offset_px: f32) {
        *self
            .pending_scroll_line
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .pending_scroll_offset
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(offset_px.max(0.0));
    }

    /// Request that the next `show` scrolls so `line` is at the top of the viewport, using the cached
    /// measured line height (or the document is rendered at least once so the height is known). If the
    /// line height has not been measured yet (no frame rendered), the request still stores a best-effort
    /// offset that is corrected on the following frame once the height is known.
    pub fn scroll_to_line(&self, line: usize) {
        let measured = *self
            .line_height_px
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(line_height) = measured {
            self.scroll_to_offset_px(line as f32 * line_height);
        } else {
            *self
                .pending_scroll_line
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(line);
            *self
                .pending_scroll_offset
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }
    }

    /// The stable AccessKit author_id for this panel's outer container, with the instance suffix when
    /// present (RISK-004).
    pub fn container_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_PANEL_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for this panel's scroll region, with the instance suffix when
    /// present (RISK-004).
    pub fn scroll_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_SCROLL_AREA_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for this panel's inner text area, with the instance suffix when
    /// present (RISK-004).
    pub fn text_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_TEXT_AUTHOR_ID)
    }

    /// Request focus on the real mounted code text node on its next frame.
    pub fn request_text_focus(&self) {
        self.editor_focus_pending
            .store(true, std::sync::atomic::Ordering::Release);
    }

    /// Read-only focus-lifecycle diagnostic for mounted integration proofs.
    #[doc(hidden)]
    pub fn text_focus_request_pending_for_test(&self) -> bool {
        self.editor_focus_pending
            .load(std::sync::atomic::Ordering::Acquire)
    }

    /// Whether the live editable TextInput node, rather than only its outer pane scope, currently
    /// owns egui focus. AccessKit targets the inner node directly, so menu/bus enablement must treat
    /// that focus as code-editor focus as well.
    pub fn live_text_has_focus(&self, ctx: &egui::Context) -> bool {
        let live_text_id = *self
            .live_text_node_id
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        live_text_id.is_some_and(|id| ctx.memory(|memory| memory.focused() == Some(id)))
    }

    /// Append the instance suffix to a base author_id (`base#instance`), or return the bare base for
    /// the default single panel (so the MT-contract ids match exactly — AC-004/AC-005).
    fn suffixed(&self, base: &str) -> String {
        if self.instance.is_empty() {
            base.to_owned()
        } else {
            format!("{base}#{}", self.instance)
        }
    }

    /// The fixed `egui::Id` for the outer container. The default panel uses the fixed `NodeId` band
    /// (200) so its live AccessKit `NodeId` is stable across frames/restarts; a multi-instance panel
    /// derives a high-entropy id from its suffixed author_id (egui's hashed id space) so two panels
    /// never share an id (RISK-004).
    fn container_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: a single hand-assigned, never-reused fixed id cannot self-collide; entropy only
            // affects egui's child IdMap distribution. 200 is disjoint from chrome (10/20/21),
            // dividers (30/31), and panes (>=100).
            unsafe { egui::Id::from_high_entropy_bits(PANEL_CONTAINER_NODE_ID) }
        } else {
            egui::Id::new(self.container_author_id())
        }
    }

    /// The fixed `egui::Id` for the scroll region (band slot 202 for the default panel; hashed for
    /// instances). See [`container_id`](Self::container_id) for the safety rationale.
    fn scroll_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_SCROLL_NODE_ID) }
        } else {
            egui::Id::new(self.scroll_author_id())
        }
    }

    /// The fixed `egui::Id` for the inner text area (band slot 201 for the default panel; hashed for
    /// instances). See [`container_id`](Self::container_id) for the safety rationale.
    fn text_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_TEXT_NODE_ID) }
        } else {
            egui::Id::new(self.text_author_id())
        }
    }

    /// Render the panel into `ui`: a virtualized, theme-colored view of the buffer's visible lines
    /// plus the three AccessKit nodes (container -> scroll-area -> text). Only the lines intersecting
    /// the viewport (plus overscan) are painted, so a 100k-line file stays within the frame budget
    /// (MT-002). Safe to call every frame; recomputes highlights only on a buffer-version change.
    pub fn show(&self, ui: &mut egui::Ui) {
        if self.poll_initial_highlight() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }
        let syntax = syntax_tokens_for(ui.visuals());
        let container_id = self.container_id();
        let container_author = self.container_author_id();
        let scroll_author = self.scroll_author_id();
        let scroll_id = self.scroll_id();
        let text_author = self.text_author_id();
        let text_id = self.text_id();
        // Keep an explicit navigation focus request pending until the actual live text node reports
        // focus. Clearing it before the node exists can lose the one-shot across a mount/snapshot pass,
        // leaving the destination tab active but keyboard-inert.
        let focus_requested = self
            .editor_focus_pending
            .load(std::sync::atomic::Ordering::Acquire);
        if focus_requested && self.live_text_has_focus(ui.ctx()) {
            self.editor_focus_pending
                .store(false, std::sync::atomic::Ordering::Release);
        }

        // MT-054: consume the Alt+Z word-wrap shortcut BEFORE the keymap dispatch / live-typing loop read
        // input (RISK-005 / MC-005). `consume_shortcut` removes the matching key event from the queue, so
        // neither `process_keymap` nor `process_cursor_input`'s Event::Text path ever sees the 'z' — the
        // toggle flips wrap WITHOUT inserting a literal 'z' into the buffer. Skipped while a rename input
        // owns the keyboard (the same focus-precedence guard `process_keymap` uses) so Alt+Z does not
        // fight the rename surface. The toggle is the SINGLE `toggle_wrap` mutation point the AccessKit
        // node also routes through (AC-005), and it is render-only (no buffer mutation — AC-007).
        if matches!(
            *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()),
            RenameState::Idle
        ) {
            let wrap_shortcut = egui::KeyboardShortcut::new(egui::Modifiers::ALT, egui::Key::Z);
            if ui.input_mut(|i| i.consume_shortcut(&wrap_shortcut)) {
                self.toggle_wrap();
            }
        }

        // Measure + cache the monospace line height once (implementation note: do it at first show
        // and reuse). `show_rows` needs the per-line height WITHOUT egui's row spacing (it adds the
        // spacing itself), and we zero item-spacing inside the rows, so the measured glyph height is
        // the row height.
        let line_height = self.line_height(ui);
        if let Some(line) = self
            .pending_scroll_line
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            *self
                .pending_scroll_offset
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(line as f32 * line_height);
        }
        // Measure + cache the monospace glyph width once, with the SAME FontId render_line paints with,
        // so the caret/selection overlay (MT-003) aligns column->x exactly (implementation note 4).
        let glyph_width = self.glyph_width(ui);

        // MT-010: poll the operator keybinding override file (~/.handshake/keymap.json) for changes and
        // reload the keymap if it moved (implementation note 6 — a throttled mtime stat, not the
        // `notify` crate). A graceful no-op when the file path is unresolvable / unchanged. Reloading
        // bumps the keymap version so the cached command nodes rebuild.
        self.maybe_reload_keymap();

        // Highlights are computed at most once per buffer version (cache hit on an unchanged buffer),
        // so the per-frame render never re-parses (MT-002 step 3).
        self.ensure_highlight_cache();
        // Fold regions are recomputed only when the buffer version moved (MT-005 impl note 3), reusing
        // the tree `ensure_highlight_cache` just parsed (no second parse). Must run AFTER the highlight
        // cache so the highlighter's tree reflects the current buffer.
        self.ensure_fold_regions();

        // Cache the document line count BEFORE the ScrollArea so it is not re-queried inside the row
        // closure (implementation note).
        let total_lines = self.with_buffer(|b| b.len_lines());

        // MT-005 step 6: the VISIBLE line count is the buffer line count minus the lines collapsed by
        // folded regions. `show_rows` is driven over the visible count (NOT `total_lines`), and the row
        // closure maps each visible row index back to a buffer line via the FoldSet. Rebuild the
        // visible->buffer map against the LIVE buffer line count once here (cheap on a fold-state cache
        // hit) so the per-row lookups in the closure are O(1) (RISK-001 / MC-001).
        let visible_lines = self
            .fold_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .rebuild_visible_map_for(total_lines);

        // Ensure the outline is current (MC-002 — reuse the highlighter's tree; recompute only on a
        // version change) before the three-panel layout reads it.
        self.ensure_outline();

        // MT-006 step 4: split the editor into a horizontal layout —
        //   [outline (optional, left)] [editor area (center)] [minimap (optional, right)].
        // The outline + minimap are nested `SidePanel`s rendered INSIDE this `ui` (the pane's rect),
        // each hideable via the toggle row (RISK-001 / MC-001 — keep the center editor usable). The
        // central editor (the existing container -> scroll -> text scope) renders in the remaining
        // space afterward, unchanged.
        let show_outline = self.is_outline_shown();
        let show_minimap = self.is_minimap_shown();

        // A slim toggle row pinned to the top of the editor pane (MC-001: the outline + minimap each
        // have a toggle button so AC-003's three-panel layout is operator-controllable). Rendered first
        // so it claims its strip; the side panels + center editor divide the remaining rect.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let outline_resp = ui
                .selectable_label(show_outline, "\u{2261} Outline")
                .on_hover_text("Toggle the outline panel");
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                outline_resp.id,
                &self.suffixed(CODE_EDITOR_TOGGLE_OUTLINE_AUTHOR_ID),
            );
            if outline_resp.clicked() {
                self.toggle_outline();
            }
            let minimap_resp = ui
                .selectable_label(show_minimap, "\u{25A4} Minimap")
                .on_hover_text("Toggle the minimap");
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                minimap_resp.id,
                &self.suffixed(CODE_EDITOR_TOGGLE_MINIMAP_AUTHOR_ID),
            );
            if minimap_resp.clicked() {
                self.toggle_minimap();
            }
            // MT-034: toggle the "Notes referencing this symbol" panel (the code->notes cross-ref
            // surface). When shown, dwelling on a symbol loads the notes that mention it (RISK-001 —
            // hideable so the center editor keeps a usable width).
            let note_refs_resp = ui
                .selectable_label(self.is_note_refs_shown(), "\u{1F4DD} Note refs")
                .on_hover_text(
                    "Toggle the panel listing notes that reference the focused code symbol",
                );
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                note_refs_resp.id,
                &self.suffixed(CODE_EDITOR_TOGGLE_NOTE_REFS_AUTHOR_ID),
            );
            if note_refs_resp.clicked() {
                self.toggle_note_refs();
            }
            // MT-054: the word-wrap toggle (Alt+Z). A visible selectable label that reflects + flips the
            // persisted WrapConfig.enabled through the SAME `toggle_wrap` mutation point Alt+Z and the
            // AccessKit node route through (AC-005). The AccessKit node itself (Role::Button, Toggled
            // property, author_id `editor-wrap-toggle`) is emitted inside the container scope below so it
            // is a container descendant a swarm agent can flip by id.
            let wrap_resp = ui
                .selectable_label(self.is_wrap_enabled(), "\u{21B5} Wrap")
                .on_hover_text("Toggle word wrap (Alt+Z)");
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                wrap_resp.id,
                &self.suffixed(CODE_EDITOR_VISIBLE_WRAP_TOGGLE_AUTHOR_ID),
            );
            if wrap_resp.clicked() {
                self.toggle_wrap();
            }
            // MT-010 'Configure keybindings' affordance: materializes ~/.handshake/keymap.json (creating
            // it with the current overrides if absent) so the operator can edit it. Deliberately does NOT
            // launch an external editor via `open::that()` — a forced app launch would steal OS focus
            // (HBR-QUIET); instead it ensures the file exists + surfaces its path in a tooltip, and the
            // per-frame hot-reload poll picks up the operator's edits. The hover shows the resolved path.
            let keymap_path_label = self
                .keymap_file_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "<home dir unavailable>".to_owned());
            let keybindings_resp = ui
                .button("\u{2328} Keybindings")
                .on_hover_text(format!("Configure editor keybindings: {keymap_path_label}"));
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                keybindings_resp.id,
                &self.suffixed(CODE_EDITOR_KEYBINDINGS_AUTHOR_ID),
            );
            if keybindings_resp.clicked() {
                self.ensure_keymap_file_exists();
            }
        });

        // OUTLINE side panel (left). `show_inside` renders within this `ui`'s rect (the pane), so the
        // panel docks to the left edge of the editor pane rather than the whole app window.
        if show_outline {
            let outline_panel_id = if self.instance.is_empty() {
                egui::Id::new("code_editor_outline_panel")
            } else {
                egui::Id::new(format!("code_editor_outline_panel#{}", self.instance))
            };
            let resp = egui::SidePanel::left(outline_panel_id)
                .resizable(true)
                .default_width(180.0)
                .show_inside(ui, |ui| {
                    self.render_outline_panel(ui, &syntax);
                });
            *self
                .last_outline_rect
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(resp.response.rect);
        } else {
            *self
                .last_outline_rect
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }

        // MT-034 NOTE-REFS side panel (right). Rendered BEFORE the minimap so the two right-edge panels
        // stack (note-refs inboard of the minimap). The panel renders the current `note_refs_state` and
        // routes a clicked row through the cross-pane Open-Document command (reuse — see
        // `render_note_refs_panel_into`). Resizable so a long note title is readable.
        if self.is_note_refs_shown() {
            let note_refs_panel_id = if self.instance.is_empty() {
                egui::Id::new("code_editor_note_refs_panel")
            } else {
                egui::Id::new(format!("code_editor_note_refs_panel#{}", self.instance))
            };
            egui::SidePanel::right(note_refs_panel_id)
                .resizable(true)
                .default_width(220.0)
                .show_inside(ui, |ui| {
                    self.render_note_refs_panel_into(ui);
                });
        }

        // MINIMAP side panel (right). Non-resizable, exact 80px (Monaco's minimap width).
        if show_minimap {
            let minimap_panel_id = if self.instance.is_empty() {
                egui::Id::new("code_editor_minimap_panel")
            } else {
                egui::Id::new(format!("code_editor_minimap_panel#{}", self.instance))
            };
            // Capture the current viewport (buffer-line space) so the minimap indicator marks the right
            // rows: the panel's last painted range is in VISIBLE-line space, so map both ends back to
            // buffer lines through the fold set. Spans are NOT cloned here — the minimap fetches them
            // internally only on a row-color cache MISS (edit/resize/theme), not every frame, so a 100k
            // span list is not copied per frame (MT-002 frame budget).
            let visible_buffer_range = self.last_painted_buffer_range(total_lines);
            // `render_minimap_panel` stores the minimap's TRUE content rect (exactly the configured
            // width) into `last_minimap_rect`; the SidePanel outer rect (frame-margin inflated) is not
            // used for geometry.
            egui::SidePanel::right(minimap_panel_id)
                .resizable(false)
                .exact_width(self.minimap.width())
                .show_inside(ui, |ui| {
                    self.render_minimap_panel(ui, visible_buffer_range, total_lines);
                });
        } else {
            *self
                .last_minimap_rect
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        }

        // OUTER container scope (the CENTER editor area). egui gives every child `Ui` its own AccessKit
        // node keyed by the `Ui`'s id and nests it under the parent `Ui`'s node. We emit the CONTAINER
        // node onto THIS scope's own `Ui` id, render the scroll-area in a nested scope inside it, and
        // render the text content nested inside that — so the live tree is container -> scroll-area ->
        // text (AC-004 + AC-005 ancestry). The fixed `container_id` is only the `id_salt` that keeps the
        // scope's id stable across frames.
        ui.scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
            let container_node_id = ui.unique_id();
            // Preserve the ancestor-bounded clip before the gutter SidePanel reserves its strip. The
            // delayed gutter pass needs this exact bound to restore pointer interaction without ever
            // expanding beyond the editor container/window.
            let editor_container_clip = ui.clip_rect();

            // Paint the editor background from the theme (no hardcoded hex).
            let bg = syntax.background;
            let full_rect = ui.available_rect_before_wrap();
            if ui.is_rect_visible(full_rect) {
                ui.painter().rect_filled(full_rect, 0.0, bg);
            }

            // MT-007: RESERVE the gutter strip on the LEFT of the center editor area BEFORE the scroll
            // area, so the editor rows start to the right of the gutter (no overlap). The strip width is
            // recomputed every frame from the LIVE buffer line count (RISK-001 / MC-001) so a
            // 99->1000-line transition widens it. The strip's actual per-row content (numbers, dots,
            // fold triangles, breakpoint circles, interactions) is painted AFTER the scroll renders,
            // once the painted-row geometry is captured — see `render_gutter` below. The SidePanel here
            // only reserves the rect + emits the Group strip node.
            let gutter_cfg = self.gutter_config();
            let gutter_glyph_width = glyph_width;
            let gutter_width =
                Gutter::width_for(total_lines, gutter_glyph_width, &gutter_cfg).max(1.0);
            let gutter_panel_id = if self.instance.is_empty() {
                egui::Id::new("code_editor_gutter_panel")
            } else {
                egui::Id::new(format!("code_editor_gutter_panel#{}", self.instance))
            };
            let gutter_author = self.gutter_author_id();
            let gutter_resp = egui::SidePanel::left(gutter_panel_id)
                .resizable(false)
                .exact_width(gutter_width)
                .show_separator_line(false)
                .show_inside(ui, |ui| {
                    // Claim the full strip height (the painter content is added after the scroll frame).
                    let strip = ui.available_rect_before_wrap();
                    ui.advance_cursor_after_rect(strip);
                    // Emit the gutter strip Group node (AC-003 / HBR-SWARM) — author_id
                    // "code_editor_gutter", role Group (exists in accesskit 0.21.1).
                    let node_id = self.gutter_node_id();
                    let author = gutter_author.clone();
                    let value = format!("{total_lines} lines");
                    ui.ctx().accesskit_node_builder(node_id, move |node| {
                        node.set_role(accesskit::Role::Group);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor gutter".to_owned());
                        node.set_value(value.clone());
                    });
                });
            let gutter_rect = gutter_resp.response.rect;
            *self
                .last_gutter_rect
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(gutter_rect);

            // MT-053: render the STICKY-SCROLL band as a pinned top strip of the CENTER editor area
            // BEFORE the scroll area, RESERVING vertical space equal to `headers.len() * line_height` so
            // the first scrolled line is never occluded (RISK-003 / MC-003). The headers are recomputed
            // every frame from the CURRENT scroll offset (the last painted buffer-line window) + the live
            // MT-005 fold regions (no caching across edits — RISK-004 / MC-004). A no-op (and no AccessKit
            // node) when no scope encloses the viewport top. Clicking a header scrolls to its scope (the
            // SAME fold-aware scroll path JumpTo uses). Rendered as a TopBottomPanel::top INSIDE this
            // center scope so it claims its strip and the scroll area divides the remaining rect — the
            // reservation is structural (the scroll area gets `available_height - band_height`), not an
            // overlay, so occlusion is impossible by construction.
            self.render_sticky_band(ui, total_lines, line_height);

            // SCROLL-AREA scope (AC-004: Role::ScrollView, author_id "code_editor_scroll_area"). The
            // virtualized rows render inside it via `show_rows`, which only invokes the closure for
            // the lines intersecting the viewport.
            ui.scope_builder(egui::UiBuilder::new().id_salt(scroll_id), |ui| {
                let scroll_node_id = ui.unique_id();

                // Zero the inter-row spacing on the SCROLL-AREA ui BEFORE calling `show_rows`. egui
                // derives its row stride as `row_height_with_spacing = line_height + item_spacing.y`
                // from THIS ui's spacing (egui 0.33.3 scroll_area.rs:943-944). Zeroing it here makes
                // the stride exactly `line_height` — the SAME sans-spacing unit `scroll_to_line` /
                // `y_for_line` / `total_height_px` use — so a requested offset of `line * line_height`
                // lands egui on exactly that row (no spacing-unit drift). `render_rows` also zeroes it
                // on its inner scope so the painted rows have no gap; doing it here too keeps egui's
                // row-index math and the pixel layout on one consistent unit. (AC-007 unit fix.)
                ui.style_mut().spacing.item_spacing.y = 0.0;

                // Consume a one-shot requested scroll offset (go-to-line / agent / test), if any.
                let pending = self
                    .pending_scroll_offset
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .take();

                let mut scroll_area = egui::ScrollArea::vertical()
                    .id_salt(("code-editor-scroll", scroll_id))
                    .auto_shrink([false, false]);
                if let Some(offset) = pending {
                    scroll_area = scroll_area.vertical_scroll_offset(offset);
                }
                // Capture the EXACT row range `show_rows` paints this frame (AC-007). egui computes it
                // INSIDE `show_rows` from the live viewport using `row_height_with_spacing = line_height
                // + item_spacing.y` and NO overscan (egui 0.33.3 scroll_area.rs:948-963), then hands it
                // to the closure as `row_range`. That painted range — not a separate VirtualLineLayout
                // recompute (which adds ±OVERSCAN_LINES egui never applies and divides by the
                // sans-spacing height) — is the authoritative diagnostics + overlay-positioning surface.
                let mut painted_range: std::ops::Range<usize> = 0..0;
                // MT-054 PERF CAP: the count of LOGICAL lines the wrap paint path byte-materialized this
                // frame. The closure writes it; it stays O(painted window) under wrap and 0 when wrap is
                // off. A perf test asserts it never approaches the document size on a large wrapped file.
                let mut frame_lines_wrapped: usize = 0;

                // MT-054: refresh the wrap config's viewport width from the LIVE editor-row width (the
                // scroll-area inner width minus a small scrollbar allowance) so `wordWrap: on` wraps at
                // the real visible edge. A no-op for the 1:1 fast path (wrap off ignores the width).
                let editor_row_width = (ui.available_width() - 16.0).max(1.0);
                {
                    let mut cfg = self.wrap_config.lock().unwrap_or_else(|e| e.into_inner());
                    cfg.viewport_width_px = editor_row_width;
                }
                let wrap_cfg = self.wrap_config();
                let wrap_enabled = wrap_cfg.enabled;
                // MT-054 PERF CAP (adversarial-review hardening): the `show_rows` row count under wrap
                // comes from the CACHED prefix-sum wrap-row index — NOT from eagerly building every
                // VisualRow in the document every frame (the O(document)/frame regression the review
                // caught). `ensure_wrap_row_index` rebuilds the index only on a key miss (edit / fold /
                // toggle / resize / metric change); a scroll / hover / idle repaint is a cache hit and
                // O(1). The per-row paint inside the closure then materializes ONLY the painted window's
                // lines (RISK-001 / MC-001 — paint + scrollbar still share ONE source of truth, the index;
                // RISK-006 / MC-006 — wrap OFF skips the index entirely so the MT-002 baseline is
                // unchanged).
                let scroll_row_count = if wrap_enabled {
                    self.ensure_wrap_row_index(visible_lines, &wrap_cfg, glyph_width)
                } else {
                    visible_lines
                };

                // MT-005: drive `show_rows` over the VISIBLE (post-fold) line count, so a folded region
                // collapses the scroll content (the scrollbar reflects the folded document). The
                // `row_range` egui hands the closure is therefore in VISIBLE-line space; `render_rows`
                // maps each visible row back to a buffer line via the FoldSet (MT step 4/6). Under wrap
                // (MT-054) the range is in VISUAL-row space and `render_wrapped_rows` maps each visual row
                // back to its logical buffer line + byte fragment.
                let scroll_output =
                    scroll_area.show_rows(ui, line_height, scroll_row_count, |ui, row_range| {
                        // Record egui's actual painted window before painting.
                        painted_range = row_range.clone();
                        if wrap_enabled {
                            // MT-054 PERF CAP: materialize ONLY the painted visual-row window's logical
                            // lines (O(window)), translated from the cached index — not the whole doc.
                            let (window_rows, window_start, lines_touched) = self
                                .wrap_rows_for_window(row_range.clone(), &wrap_cfg, glyph_width);
                            frame_lines_wrapped = lines_touched;
                            self.render_wrapped_rows(
                                ui,
                                &window_rows,
                                window_start,
                                &syntax,
                                total_lines,
                                text_id,
                                &text_author,
                                line_height,
                                glyph_width,
                            );
                        } else {
                            self.render_rows(
                                ui,
                                row_range,
                                &syntax,
                                total_lines,
                                visible_lines,
                                text_id,
                                &text_author,
                                line_height,
                                glyph_width,
                            );
                        }
                    });

                // Store egui's actual painted row range as BOTH the perf "lines painted this frame"
                // count and the `last_visible_range` overlay seam (AC-007). The painted range is the
                // ground truth MT-003+ reads to position the cursor/gutter/selection overlay, so the
                // diagnostics must equal it exactly — not the overscan-padded calculator estimate.
                //
                // ORDER MATTERS (wave-2 remediation of the wave-1 MAJOR "one-frame-stale input
                // mapping"): this store MUST happen BEFORE `process_cursor_input` below, because
                // `pointer_to_byte` resolves the clicked row via `last_visible_range().start` while
                // `row_geometry` is already CURRENT-frame — storing the range after input processing
                // made a scroll+click-in-one-frame land the caret on the PREVIOUS frame's top row
                // (off by exactly the rows scrolled that frame).
                let stats = PerfStats {
                    frame_lines_rendered: painted_range.len(),
                    buffer_len_lines: total_lines,
                    frame_lines_wrapped,
                };
                *self.perf.lock().unwrap_or_else(|e| e.into_inner()) = stats;
                *self
                    .last_visible_range
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = painted_range.clone();
                *self
                    .last_scroll_offset_px
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = scroll_output.state.offset.y.max(0.0);

                // MT-003: process multi-cursor input AFTER the rows painted this frame, so the captured
                // row geometry is available to map a pointer position (Alt+Click / box drag) to a
                // (line, col) byte offset — and AFTER the painted range was stored above, so the
                // pointer->row mapping uses THIS frame's window, never last frame's. Reads egui input
                // events from this scroll scope's `ui`.
                self.process_cursor_input(ui, line_height, glyph_width, total_lines);

                // Emit the ScrollView node onto THIS scroll scope's Ui id (AC-004). It is a child of
                // the container scope and the parent of the text scope.
                let author = scroll_author.clone();
                ui.ctx()
                    .accesskit_node_builder(scroll_node_id, move |node| {
                        node.set_role(accesskit::Role::ScrollView);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor scroll area".to_owned());
                        node.set_value(format!(
                            "{} of {} lines rendered",
                            stats.frame_lines_rendered, stats.buffer_len_lines
                        ));
                    });
            });

            // MT-007: paint the gutter strip content NOW — after the scroll area painted its rows, so
            // the captured `RowGeometry` (origin/line_height) is current and the gutter aligns row-for-
            // row with the editor body (including under MT-005 folds — the per-row buffer-line list is
            // taken from the SAME fold-mapped visible window). Applies any fold/breakpoint click to the
            // panel state and publishes a BreakpointEvent on a breakpoint toggle. Nested in the container
            // scope so the gutter's per-line breakpoint/diagnostic nodes are container descendants.
            self.render_gutter(
                ui,
                gutter_rect,
                gutter_glyph_width,
                &gutter_cfg,
                editor_container_clip,
            );

            // MT-048/MT-070: install the editor-body secondary-click surface AFTER the body and gutter
            // responses have been registered. egui resolves an overlapping interaction to the latest
            // response in the layer; registering this before the virtualized rows left the advertised
            // AccessKit surface addressable while its real secondary click was shadowed by the later row
            // widgets. The editor's primary caret/gutter handlers have already consumed this frame's
            // input above, while the find bar is rendered after this and therefore remains topmost over
            // its own controls.
            let editor_body_context_rect = egui::Rect::from_min_max(
                egui::pos2(gutter_rect.right(), full_rect.top()),
                full_rect.max,
            );
            self.render_editor_context_menu(ui, editor_body_context_rect);

            // MT-004: render the floating find bar (Ctrl+F / Ctrl+H) pinned to the top-right of the
            // editor area, INSIDE the container scope so its AccessKit nodes are descendants of the
            // container (the same nesting the scroll/text nodes use). A no-op when the bar is closed.
            self.render_find_bar(ui, full_rect, &syntax);

            // MT-010: emit the hidden editor-command AccessKit nodes (one Role::Button per
            // CodeEditorAction, author_id code_editor_cmd_*) INSIDE the container scope so they are
            // container descendants like the scroll/text/fold nodes. They have no visual area (invisible
            // to the operator) but are addressable by a swarm agent / MCP tool to dispatch any editor
            // command without a keystroke (AC-005 / HBR-SWARM). The descriptor set is cached per keymap
            // version (RISK-002 — built once per keymap change, not every frame).
            self.emit_command_nodes(ui);

            // MT-054: emit the word-wrap toggle AccessKit node (author_id `editor-wrap-toggle`,
            // Role::Button with a Toggled property reflecting the persisted WrapConfig.enabled), INSIDE
            // the container scope so it is a container descendant a swarm agent can flip by id (AC-005 /
            // HBR-SWARM). Attached under the MT-002 editor container — NOT a second editor root node.
            self.emit_wrap_toggle_node(ui);

            // Emit the container node onto this scope's Ui id from INSIDE the scope, so it is the
            // node that parents the nested scroll-area scope (AC-005: GenericContainer + author_id).
            let author = container_author.clone();
            ui.ctx()
                .accesskit_node_builder(container_node_id, move |node| {
                    node.set_role(accesskit::Role::GenericContainer);
                    node.set_author_id(author.clone());
                    node.set_label("Code editor".to_owned());
                });
        });

        // MT-006: render the go-to-line palette as a centered modal overlay (Ctrl+G). A no-op (and no
        // AccessKit node) when the palette is closed (AC-005). Rendered AFTER the editor scope so it
        // floats above the editor rows.
        self.render_goto_line_modal(ui, &syntax);

        // MT-053: render the in-file Go to Symbol palette as a centered modal overlay (Ctrl+Shift+O). A
        // no-op (and no AccessKit node) when closed (AC-003). Rendered AFTER the editor scope so it floats
        // above the rows, like the go-to-line palette.
        self.render_symbol_palette_modal(ui, &syntax);

        // MT-008 LIVE loop: pump the code-intelligence triggers from the running frame — drain LSP
        // diagnostics onto the gutter (AC-008), advance the hover dwell + fire a hover lookup on a dwell
        // hit, and fire a completion lookup when one was armed this frame by `process_cursor_input`
        // (Ctrl+Space / trigger char). Runs AFTER `process_cursor_input` (so the caret offset is current)
        // and BEFORE the overlay render below (so a result delivered last frame paints this frame). A
        // graceful no-op without an injected runtime / bound workspace.
        self.pump_code_intelligence();

        // MT-034 LIVE code->notes loop: advance the cursor-dwell debounce and, on a dwell crossing, fire
        // the find-notes search off-thread (RISK-3 / MC-3 — once per dwell, never per frame); then drain
        // any result delivered last frame into `note_refs_state` so the NoteRefsPanel (rendered above as a
        // right SidePanel) shows it. Both are graceful no-ops when the panel is hidden / no runtime /
        // workspace, so a headless harness renders cleanly. The drain runs every frame so a result that
        // landed while the panel was briefly hidden is still picked up when it re-shows.
        self.pump_note_refs();
        self.drain_note_refs();

        // MT-049 LIVE quick-fix loop: install the result receiver (once), drain any delivered code-action
        // result onto the controller (lights the bulb / opens the menu), fire a Ctrl+./context-menu request
        // when armed, and advance the cursor-rest debounce to fire a passive request on a diagnostic line
        // (RISK-001 / MC-001 — once per dwell, never per idle frame). A graceful no-op without a runtime.
        self.pump_code_actions();

        // MT-050 LIVE format loop: drain any delivered format result (install the formatted text as one
        // undo step + surface the error toast), then fire an armed Alt+Shift+F / context-menu format request
        // off-thread (HBR-QUIET — the egui frame never blocks on the LSP). A graceful no-op without a runtime.
        self.pump_formatting();

        // MT-008: drain any off-thread code-nav/LSP results into the popup state, then render the
        // completion popup + hover tooltip as non-focus-stealing overlays ABOVE the editor (RISK-005).
        // A no-op (and no AccessKit nodes) when neither is open (AC-005/AC-006).
        self.render_code_intelligence(ui);

        // The stable `text_id` above identifies the enclosing scope, while the actual AccessKit
        // TextInput node is the nested live id recorded by `render_rows`/`render_wrapped_rows`.
        // Focusing the scope id leaves AccessKit's focused id absent from the emitted node list as
        // soon as another surface renders. Apply the one-shot focus request only after the live node
        // exists so tab/menu navigation never publishes a dangling focused node.
        if let Some(live_text_id) = *self
            .live_text_node_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        {
            if focus_requested {
                ui.ctx()
                    .memory_mut(|memory| memory.request_focus(live_text_id));
                ui.ctx().request_repaint();
            }
        }
    }

    fn completion_request_is_current(&self, request: &CodeIntelligenceRequestIdentity) -> bool {
        request.generation == self.completion_generation.load(Ordering::Relaxed)
            && self.code_intelligence_state_is_current(request)
    }

    fn current_code_intelligence_identity(
        &self,
        generation: u64,
        query: String,
    ) -> CodeIntelligenceRequestIdentity {
        CodeIntelligenceRequestIdentity {
            generation,
            buffer_version: self.buffer_version.load(Ordering::Relaxed),
            cursor_byte: self.primary_cursor_offset(),
            document_uri: self.lsp_uri(),
            workspace_id: self.workspace_id(),
            query,
        }
    }

    fn cancel_automatic_completion(&self) {
        let _ = self.completion_request.compare_exchange(
            COMPLETION_REQUEST_AUTOMATIC,
            COMPLETION_REQUEST_NONE,
            Ordering::Relaxed,
            Ordering::Relaxed,
        );
        *self
            .automatic_completion_cursor
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    fn invalidate_stale_code_intelligence_overlays(&self) {
        let completion_stale = self
            .completion_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|identity| !self.completion_request_is_current(identity))
            .unwrap_or(false);
        if completion_stale {
            self.close_completion();
        }
        let hover_stale = self
            .hover_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|identity| !self.hover_request_is_current(identity))
            .unwrap_or(false);
        if hover_stale {
            self.close_hover();
        }
    }

    fn hover_request_is_current(&self, request: &CodeIntelligenceRequestIdentity) -> bool {
        request.generation == self.hover_generation.load(Ordering::Relaxed)
            && self.code_intelligence_state_is_current(request)
    }

    fn definition_request_is_current(&self, request: &CodeIntelligenceRequestIdentity) -> bool {
        request.generation == self.definition_generation.load(Ordering::Relaxed)
            && self.code_intelligence_state_is_current(request)
    }

    fn references_request_is_current(&self, request: &CodeIntelligenceRequestIdentity) -> bool {
        request.generation == self.references_generation.load(Ordering::Relaxed)
            && self.code_intelligence_state_is_current(request)
    }

    fn code_intelligence_state_is_current(
        &self,
        request: &CodeIntelligenceRequestIdentity,
    ) -> bool {
        request.buffer_version == self.buffer_version.load(Ordering::Relaxed)
            && request.cursor_byte == self.primary_cursor_offset()
            && request.document_uri == self.lsp_uri()
            && request.workspace_id == self.workspace_id()
    }

    /// MT-008: drain the off-thread completion/hover result cells into the popup state and render the
    /// completion popup + hover tooltip overlays. Both are non-focus-stealing `egui::Area`s on the
    /// Foreground order (RISK-005 — they never take the editor's keyboard, so opening the popup never
    /// drops a character). A click on a completion item inserts it; a click on the hover go-to-def link
    /// navigates. Emits the `code_editor_completion_popup` ListBox + `code_editor_completion_item_{n}`
    /// Option nodes (AC-005) and the `code_editor_hover` Tooltip node (AC-006).
    fn render_code_intelligence(&self, ui: &egui::Ui) {
        // Drain delivered completion items into the popup state (HBR-QUIET — the spawn delivered them
        // off-thread; here we just swap them in on the UI thread).
        let delivered_symbol_batches = {
            let mut guard = self
                .code_nav_symbols_result
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *guard)
        };
        let had_symbol_batches = !delivered_symbol_batches.is_empty();
        let mut delivered_symbols = Vec::new();
        for (workspace_id, prefix, symbols) in delivered_symbol_batches {
            delivered_symbols.extend(symbols.iter().cloned());
            self.code_nav_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .put(workspace_id, prefix, symbols);
        }
        if had_symbol_batches {
            self.push_staleness_markers(&delivered_symbols);
        }
        if let Some(delivery) = self
            .completion_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            if self.completion_request_is_current(&delivery.request) {
                if let Some((prefix, symbols)) = delivery.code_nav_batch {
                    self.code_nav_cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .put(&delivery.request.workspace_id, prefix, symbols.clone());
                    self.push_staleness_markers(&symbols);
                }
                if delivery.items.is_empty() {
                    self.close_completion();
                } else {
                    self.reset_completion_observer_for_popup();
                    let visible_identity = delivery.request.clone();
                    *self
                        .completion_state
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) =
                        Some(CompletionState::new(delivery.items, delivery.anchor));
                    *self
                        .completion_visible_identity
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(visible_identity);
                }
            }
        }
        // Drain a delivered hover result.
        if let Some(delivery) = self
            .hover_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            if self.hover_request_is_current(&delivery.request) {
                if let Some((prefix, symbols)) = delivery.code_nav_batch {
                    self.code_nav_cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .put(&delivery.request.workspace_id, prefix, symbols.clone());
                    self.push_staleness_markers(&symbols);
                }
                if let Some(hover) = delivery.hover {
                    *self.hover_state.lock().unwrap_or_else(|e| e.into_inner()) = Some(hover);
                    *self
                        .hover_visible_identity
                        .lock()
                        .unwrap_or_else(|e| e.into_inner()) = Some(delivery.request);
                } else {
                    self.close_hover();
                }
            }
        }
        // Drain a delivered go-to-definition target (F12): jump the caret + scroll to the def line.
        if let Some(delivery) = self
            .goto_def_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            if self.definition_request_is_current(&delivery.request) {
                *self
                    .last_definition_target
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = delivery.target.clone();
                if let Some(target) = delivery.target {
                    self.apply_code_navigation_target_from_origin(target, delivery.origin_pane);
                }
            }
        }
        // Drain a delivered references result (Shift+F12) into one actionable overlay regardless of
        // whether LSP or CodeNav produced the locations.
        if let Some(delivery) = self
            .references_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            if self.references_request_is_current(&delivery.request) {
                match delivery.payload {
                    ReferencesPayload::Lsp(locations) => {
                        if locations.is_empty() {
                            self.close_references();
                        } else {
                            let items = locations
                                .iter()
                                .cloned()
                                .map(|target| CodeReferenceItem {
                                    label: target
                                        .path
                                        .clone()
                                        .unwrap_or_else(|| target.uri.clone()),
                                    target,
                                })
                                .collect();
                            *self
                                .last_lsp_references
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = locations;
                            *self
                                .last_references
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = None;
                            *self
                                .reference_items
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = items;
                            *self
                                .references_visible_identity
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = Some(delivery.request);
                        }
                    }
                    ReferencesPayload::CodeNav { raw, items } => {
                        if items.is_empty() {
                            self.close_references();
                        } else {
                            tracing::debug!(
                                total = raw.total(),
                                "code editor: CodeNav references delivered"
                            );
                            *self
                                .last_references
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = Some(raw);
                            self.last_lsp_references
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .clear();
                            *self
                                .reference_items
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = items;
                            *self
                                .references_visible_identity
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = Some(delivery.request);
                        }
                    }
                }
            }
        }

        // Render the completion popup (a no-op when closed). The panel owns the state; the popup is a
        // stateless renderer that returns the click outcome.
        if let Some(state) = self.completion_state() {
            let observer = self.completion_observer_snapshot();
            let observer_author_id = self.completion_observer_author_id();
            match CompletionPopup::show(
                ui.ctx(),
                &state,
                &self.instance,
                &observer.context,
                observer.generation,
                &observer_author_id,
            ) {
                CompletionOutcome::Accept(index) => {
                    let semantic_value =
                        state.items.get(index).map(|item| item.insert_text.clone());
                    let version_before = self.buffer_version.load(Ordering::Relaxed);
                    if self.accept_completion_index(index)
                        && self.buffer_version.load(Ordering::Relaxed) > version_before
                    {
                        if let Some(semantic_value) = semantic_value {
                            self.mark_completion_click_applied(
                                &observer.context,
                                observer.generation,
                                index,
                                &semantic_value,
                            );
                        }
                    }
                }
                CompletionOutcome::Dismiss => self.close_completion(),
                CompletionOutcome::None => {}
            }
        }
        self.emit_completion_observer(ui.ctx());

        // Render the hover tooltip (a no-op when closed).
        if let Some(state) = self.hover_state() {
            match HoverTooltip::show(ui.ctx(), &state, &self.instance) {
                HoverOutcome::GotoDefinition(target) => {
                    self.apply_code_navigation_target(target);
                    self.close_hover();
                }
                HoverOutcome::None => {}
            }
        }

        let references = self
            .reference_items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if !references.is_empty() {
            let mut clicked_reference = None;
            let mut dismiss = false;
            egui::Area::new(egui::Id::new(self.suffixed("code_editor_references")))
                .order(egui::Order::Foreground)
                .fixed_pos(egui::pos2(24.0, 72.0))
                .show(ui.ctx(), |ui| {
                    egui::Frame::popup(ui.style()).show(ui, |ui| {
                        let header = ui.strong(format!("References ({})", references.len()));
                        let overlay_author = self.suffixed("code_editor_references");
                        let reference_count = references.len();
                        ui.ctx().accesskit_node_builder(header.id, move |node| {
                            node.set_role(accesskit::Role::List);
                            node.set_author_id(overlay_author.clone());
                            node.set_label("Code references".to_owned());
                            node.set_value(format!("{reference_count} references"));
                        });
                        for (index, item) in references.iter().take(20).enumerate() {
                            let link = ui.link(format!(
                                "{} — {}:{}",
                                item.label,
                                item.target.range.start.line + 1,
                                item.target.range.start.character + 1
                            ));
                            let author = self.reference_author_id(index);
                            ui.ctx().accesskit_node_builder(link.id, move |node| {
                                node.set_role(accesskit::Role::Link);
                                node.set_author_id(author.clone());
                                node.set_label("Open reference".to_owned());
                                node.add_action(accesskit::Action::Click);
                            });
                            if link.clicked() {
                                clicked_reference = Some(index);
                            }
                        }
                        let close = ui.button("Close");
                        let close_author = self.suffixed("code_editor_references_close");
                        ui.ctx().accesskit_node_builder(close.id, move |node| {
                            node.set_role(accesskit::Role::Button);
                            node.set_author_id(close_author.clone());
                            node.set_label("Close references".to_owned());
                            node.add_action(accesskit::Action::Click);
                        });
                        dismiss = close.clicked();
                    });
                });
            if let Some(index) = clicked_reference {
                self.activate_reference(index);
            } else if dismiss {
                self.close_references();
            }
        }

        // MT-047: drain a delivered signature-help result into the popup state. A delivered result that
        // anchors to a call site the cursor has since left is dropped (the cursor-exit check) so a stale
        // popup does not linger.
        if let Some(state) = self
            .signature_help_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
        {
            // Only show it if the cursor is still inside the same call (RISK-002 — the anchor must still
            // enclose the caret). Otherwise the call ended while the request was in flight; drop it.
            let cursor_byte = self.primary_cursor_offset();
            if self.active_call_open_paren(cursor_byte) == Some(state.anchor_byte) {
                self.open_signature_help(state);
            } else {
                self.close_signature_help();
            }
        }

        // Render the signature-help popup (a no-op when closed). It is a non-focus-stealing Tooltip-order
        // Area above the cursor line (RISK-003/006), with the active parameter emphasized (AC-004) and a
        // Role::Tooltip AccessKit node `code_editor_signature_help` carrying the active label (AC-005).
        if let Some(state) = self.signature_help_state() {
            if let Some(anchor) = self.cursor_screen_pos() {
                render_signature_popup(ui.ctx(), &state, anchor, &self.instance);
            }
        }

        // MT-048: drain a delivered off-thread rename result into the rename state, then render the rename
        // surface (inline input / multi-file preview / error). Both are no-ops when rename is Idle.
        self.drain_rename_result();
        self.render_rename(ui);

        // MT-049: render the quick-fix popup menu (a no-op when the menu is closed). The lightbulb itself is
        // drawn in the gutter (`render_gutter`); this renders the menu the lightbulb / Ctrl+. / context-menu
        // entry opened, and handles the menu's keyboard verbs (arrows move selection, Enter applies, Escape
        // closes).
        self.render_quickfix_menu(ui);
    }

    /// MT-049: render the quick-fix popup menu for the current action list (a no-op when the menu is
    /// closed). Lists each action title; arrow keys move the selection, Enter applies the selected action,
    /// Escape closes. On apply it calls [`apply_quickfix`](Self::apply_quickfix) (which delegates to the
    /// MT-048 apply path). When the list is empty (the Ctrl+. degraded path) the menu shows "No quick fixes
    /// available" and closes (AC-005). Emits the `Role::Menu` + `Role::MenuItem` AccessKit nodes (AC-004).
    fn render_quickfix_menu(&self, ui: &egui::Ui) {
        // Snapshot the state so the controller lock is not held across the render closures.
        let state = {
            let guard = self
                .code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            match guard.state() {
                Some(s) if s.menu_open => s.clone(),
                _ => return, // menu closed -> nothing to render (AC-005/AC-006: no node when closed).
            }
        };
        // Anchor the menu at the action line's gutter position (or the cursor for a Ctrl+. with no line).
        let anchor = self
            .quickfix_line_screen_pos(state.line)
            .or_else(|| self.cursor_screen_pos())
            .unwrap_or(egui::pos2(40.0, 40.0));
        let menu_action = code_actions::render_menu(ui.ctx(), &state, anchor, &self.instance);

        // The menu's keyboard verbs: Up/Down move the selection, Enter applies, Escape closes.
        let (up, down, enter, escape) = ui.input(|i| {
            (
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if up {
            self.code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .select_prev();
        }
        if down {
            self.code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .select_next();
        }

        match menu_action {
            MenuAction::Apply(index) => {
                self.code_action_controller
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .select_index(index);
                let _ = self.apply_quickfix();
            }
            MenuAction::Close => self.close_quickfix_menu(),
            MenuAction::None => {
                if escape {
                    self.close_quickfix_menu();
                } else if enter && !state.actions.is_empty() {
                    let _ = self.apply_quickfix();
                }
            }
        }
    }

    /// MT-048: render the rename surface for the current [`RenameState`] (a no-op when Idle):
    /// - `Editing`  -> the inline rename input at the identifier, pre-filled + select-all on open; Enter
    ///   confirms, Escape cancels.
    /// - `Previewing` -> the multi-file WorkspaceEdit preview window (Apply/Cancel + the no-LSP banner when
    ///   it is a single-file fallback). Apply applies the preview; Cancel returns to Idle.
    /// - `Error` -> a small error frame; Escape/click dismisses.
    fn render_rename(&self, ui: &egui::Ui) {
        // Snapshot the phase so we do not hold the rename_state lock across the render closures.
        let phase = self.rename_state();
        match phase {
            RenameState::Idle => {}
            RenameState::Editing { ident_range, .. } => {
                // Anchor the input at the identifier's screen position (the start of the identifier).
                let (line, col) = self.with_buffer(|b| byte_to_line_col(ident_range.start, b));
                let glyph_width = (*self
                    .glyph_width_px
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()))
                .unwrap_or(8.0);
                let anchor = self
                    .screen_pos_for_line_col(line, col, glyph_width)
                    .unwrap_or(egui::pos2(20.0, 20.0));
                // Render against a mutable copy of the state so the input edits the draft, then write back.
                let mut state = self.rename_state();
                rename::render_inline_input(ui.ctx(), &mut state, anchor, &self.instance);
                // Persist the edited draft + the (now-consumed) one-shot focus flag.
                self.set_rename_state(state);
                // Read Enter (confirm) / Escape (cancel) from the frame's key events. The input is a
                // singleline TextEdit, so Enter is delivered as a Key event (not inserted), and Escape too.
                let (enter, escape) = ui.input(|i| {
                    (
                        i.key_pressed(egui::Key::Enter),
                        i.key_pressed(egui::Key::Escape),
                    )
                });
                if escape {
                    self.cancel_rename();
                } else if enter {
                    match self.runtime_handle() {
                        Some(rt) => self.confirm_rename(&rt),
                        // No runtime (headless harness): the LSP/off-thread path is unavailable, so confirm
                        // synchronously via the single-file fallback so the deterministic path still works.
                        None => self.confirm_rename_sync_fallback(),
                    }
                }
            }
            RenameState::Previewing { workspace_edit } => {
                let action = rename::render_preview(ui.ctx(), &workspace_edit, &self.instance);
                match action {
                    PreviewAction::Apply => {
                        let _ = self.apply_rename_preview();
                    }
                    PreviewAction::Cancel => self.cancel_rename(),
                    PreviewAction::None => {
                        // Escape also cancels the preview.
                        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                            self.cancel_rename();
                        }
                    }
                }
            }
            RenameState::Error { message } => {
                let area_id = egui::Id::new(("code-editor-rename-error", &self.instance));
                egui::Area::new(area_id)
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 40.0))
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style()).show(ui, |ui| {
                            ui.colored_label(ui.visuals().error_fg_color, &message);
                            if ui.button("Dismiss").clicked() {
                                self.cancel_rename();
                            }
                        });
                    });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.cancel_rename();
                }
            }
        }
    }

    /// MT-048: the synchronous single-file fallback used when no tokio runtime is injected (a headless
    /// kittest harness): resolve the in-file occurrences from tree-sitter + build the single-file preview
    /// directly, with the no-LSP banner. This keeps the deterministic input->preview->apply path provable
    /// WITHOUT a runtime / live PG (the MT proof discipline). The live path uses [`confirm_rename`] +
    /// the off-thread LSP request.
    fn confirm_rename_sync_fallback(&self) {
        let (original, draft) = {
            let guard = self.rename_state.lock().unwrap_or_else(|e| e.into_inner());
            match &*guard {
                RenameState::Editing {
                    original, draft, ..
                } => (original.clone(), draft.clone()),
                _ => return,
            }
        };
        let new_name = draft.trim().to_owned();
        if new_name.is_empty() || new_name == original {
            self.cancel_rename();
            return;
        }
        let buffer_text = self.with_buffer(|b| b.to_string());
        let file_uri = self
            .lsp_uri()
            .unwrap_or_else(|| format!("file:///{}", self.file_path().trim_start_matches('/')));
        let occurrences = self.identifier_occurrences_in_buffer(&original);
        let preview = WorkspaceEditPreview::single_file_fallback(
            file_uri,
            &buffer_text,
            &new_name,
            &occurrences,
            true,
        );
        *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = RenameState::Previewing {
            workspace_edit: preview,
        };
    }

    /// MT-070 (wave-2 wiring): the editor body context menu, now rendered through the TYPED
    /// `context_menu_surfaces::editor_body_context_items` layer instead of the old inline 2-entry menu.
    /// A secondary-click over `rect` opens the 5-entry MT-070 menu (Rename Symbol / Quick Fix / Format
    /// Selection / Peek Definition / Create note from link) plus the MT-046 'Copy as note reference'
    /// entry; each confirmed id maps back through `editor_body_action_for_id` (a disabled/dead entry can
    /// never fire — AC-070-5) and dispatches the SAME live panel path its keybinding uses:
    /// - Rename Symbol -> `begin_rename_at_cursor` (F2 path, MT-048),
    /// - Quick Fix -> arms `quick_fix_request` (Ctrl+. path, MT-049),
    /// - Format Selection -> `request_format_selection` (MT-050),
    /// - Peek Definition -> `request_go_to_definition` (F12 path, MT-008),
    /// - Create note from link -> stages the typed MT-057 create-note intent
    ///   ([`take_pending_create_note_link`](Self::take_pending_create_note_link)),
    /// - Copy as note reference -> stages the `[[code:…]]` ref the factory render writes to the SHARED
    ///   InteractionBus clipboard (MT-046).
    ///
    /// Availability is read FRESH from the live panel at right-click time (RISK-070-1 — a stale snapshot
    /// never enables a dead entry). Also emits the always-present `code_editor_ctx_rename_symbol` /
    /// `code_editor_ctx_quick_fix` MenuItem AccessKit nodes (AC-005 / HBR-SWARM) exactly as before.
    fn render_editor_context_menu(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        // A secondary-click sensing response over the editor body so `context_menu` can open on it.
        let resp = ui.interact(
            rect,
            ui.id().with(("code-editor-ctx-menu", &self.instance)),
            egui::Sense::click(),
        );
        let context_author = self.suffixed(CODE_EDITOR_CONTEXT_SURFACE_AUTHOR_ID);
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_role(accesskit::Role::GenericContainer);
            node.set_author_id(context_author.clone());
            node.set_label("Code editor context surface".to_owned());
        });
        if resp.secondary_clicked() {
            self.context_menu_open_for_snapshot
                .store(true, Ordering::Relaxed);
        }
        if self.snapshot_capture_mode.load(Ordering::Relaxed)
            && self.context_menu_open_for_snapshot.load(Ordering::Relaxed)
        {
            crate::context_menu::request_open(ui.ctx(), resp.id, rect.center());
        }

        // Live availability + the typed MT-070 item list (the ids ARE the stable author_ids the owning
        // MTs emit — no parallel id scheme), extended by the MT-046 copy-as-note-reference entry.
        let availability = self.editor_body_availability();
        let can_copy_ref = self.note_reference_for_cursor().is_some();
        let copy_ref_item = {
            let item = crate::context_menu::ContextMenuItem::action(
                CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID,
                "Copy as note reference",
            );
            if can_copy_ref {
                item
            } else {
                item.disabled("No selection or identifier under the cursor to reference")
            }
        };
        let menu = crate::context_menu::ContextMenu::new("editor-body")
            .items(crate::context_menu_surfaces::editor_body_context_items(
                availability,
            ))
            .separator()
            .item(copy_ref_item);
        if let Some(confirmed) = menu.show_on(&resp) {
            self.context_menu_open_for_snapshot
                .store(false, Ordering::Relaxed);
            if confirmed == CODE_EDITOR_CTX_COPY_NOTE_REF_AUTHOR_ID {
                // MT-046: build the `[[code:…]]` ref from the live selection/identifier and stage it
                // for the factory render's bus clipboard write (the REAL command, not a fabricated
                // payload). The same one-path rule as the keymap dispatch (`CopyAsNoteReference`).
                self.copy_as_note_reference();
            } else if let Some(action) =
                crate::context_menu_surfaces::editor_body_action_for_id(confirmed, availability)
            {
                use crate::context_menu_surfaces::EditorBodyMenuAction as B;
                match action {
                    B::RenameSymbol => self.begin_rename_at_cursor(),
                    // MT-049 (AC-007): the SAME request+open_menu flow as Ctrl+. — arms the quick-fix
                    // request the per-frame pump fires for the caret line (no duplicate apply logic).
                    B::QuickFix => self.quick_fix_request.store(true, Ordering::Relaxed),
                    B::FormatSelection => self.request_format_selection(),
                    B::PeekDefinition => self.request_go_to_definition(),
                    B::CreateNoteFromLink => self.stage_create_note_from_link(),
                }
            }
        }

        // Always-present MenuItem AccessKit node carrying the exact contract author_id (AC-005). A swarm
        // agent reads/activates it by id; the value names the action so a no-context model knows what it
        // does. Emitted on a fixed node id in the rename overlay band, distinct from the popup nodes.
        let author = if self.instance.is_empty() {
            rename::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID.to_owned()
        } else {
            format!(
                "{}#{}",
                rename::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
                self.instance
            )
        };
        let node_id = if self.instance.is_empty() {
            // SAFETY: a single hand-assigned fixed id (715) in the disjoint rename overlay band (above the
            // 710..714 rename popup nodes); never reused, cannot self-collide.
            unsafe { egui::Id::from_high_entropy_bits(715) }
        } else {
            egui::Id::new(format!(
                "{}#{}",
                rename::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID,
                self.instance
            ))
        };
        ui.ctx().accesskit_node_builder(node_id, move |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_author_id(author.clone());
            node.set_label("Rename Symbol".to_owned());
            node.set_value("Open the code-editor context menu at Rename Symbol".to_owned());
            node.add_action(accesskit::Action::Click);
        });
        let open_context_menu = ui.input(|input| {
            input
                .accesskit_action_requests(node_id, accesskit::Action::Click)
                .next()
                .is_some()
        });
        if open_context_menu {
            // Canonical Argus path: clicking the stable always-mounted node opens the REAL typed context
            // popup. The caller then re-inspects and clicks the actual
            // `ctx-menu.code_editor_ctx_rename_symbol` leaf; no synthetic direct rename bypass exists.
            crate::context_menu::request_open(ui.ctx(), resp.id, rect.center());
            self.context_menu_open_for_snapshot
                .store(true, Ordering::Relaxed);
            ui.ctx().request_repaint();
        }

        // MT-049 (AC-007 / HBR-SWARM): the always-addressable 'Quick Fix...' context-menu node. A swarm
        // agent reads/activates it by `code_editor_ctx_quick_fix` to arm the SAME request+open_menu flow as
        // Ctrl+. (no duplicate apply logic). Emitted EVERY frame so the swarm surface is always present.
        let qf_author = code_actions::scoped_author_id(
            code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID,
            &self.instance,
        );
        let qf_node_id = if self.instance.is_empty() {
            // SAFETY: a single hand-assigned fixed id (721) in the disjoint quick-fix band (720 = the menu
            // container; 730.. = lightbulbs; 760.. = items), never reused.
            unsafe { egui::Id::from_high_entropy_bits(721) }
        } else {
            egui::Id::new(format!(
                "{}#{}",
                code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID,
                self.instance
            ))
        };
        ui.ctx().accesskit_node_builder(qf_node_id, move |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_author_id(qf_author.clone());
            node.set_label("Quick Fix".to_owned());
            node.set_value(
                "Show code actions / quick fixes for the current line (Ctrl+.)".to_owned(),
            );
            node.add_action(accesskit::Action::Click);
        });
    }

    /// Reproduce a previously opened dynamic editor-body menu during a canonical fresh-tree capture.
    /// The app brackets only its side-effect-free MCP snapshot frame with this flag.
    pub fn set_snapshot_capture_mode(&self, enabled: bool) {
        self.snapshot_capture_mode.store(enabled, Ordering::Relaxed);
    }

    /// MT-070: the LIVE availability of each editor-body context-menu action for the CURRENT
    /// caret/selection, read fresh at right-click time (RISK-070-1). Drives honest enable/disable:
    /// - `symbol_under_cursor`: a tree-sitter identifier at the primary caret (the same
    ///   `rename::identifier_range_at` resolution the F2 path uses),
    /// - `quick_fix_available`: actions already resolved on the caret line (the lightbulb state) OR a
    ///   live runtime that can discover them on request (the Ctrl+. arm-then-pump path),
    /// - `has_selection`: a non-empty primary selection (Format Selection's target),
    /// - `definition_available`: the F12 request's own gates (runtime + bound workspace + a word under
    ///   the caret — without all three `request_go_to_definition` is a silent no-op, so the entry is
    ///   honestly disabled instead of dead-but-enabled),
    /// - `unresolved_link_under_cursor`: a `[[title]]` under the caret that the successfully seeded
    ///   workspace resolver confirms is unresolved. Unknown resolver state fails closed.
    fn editor_body_availability(&self) -> crate::context_menu_surfaces::EditorBodyAvailability {
        // Ensure the highlight tree reflects the current buffer before resolving (cache hit when
        // unchanged) — the same freshness rule `begin_rename_at_cursor` applies.
        self.ensure_highlight_cache();
        let cursor_byte = self.primary_cursor_offset();
        let symbol_under_cursor = {
            let highlighter = self.highlighter.lock().unwrap_or_else(|e| e.into_inner());
            highlighter
                .as_ref()
                .and_then(|hl| hl.tree())
                .and_then(|tree| rename::identifier_range_at(tree, cursor_byte))
                .is_some()
        };
        let cursor_line = self.primary_cursor_line();
        let quick_fix_available =
            self.has_quickfix_on_line(cursor_line) || self.runtime_handle().is_some();
        let has_selection = self.selected_primary_text().is_some();
        let definition_available = self.runtime_handle().is_some()
            && !self.workspace_id().is_empty()
            && !self.word_at_primary_cursor().is_empty();
        let unresolved_link_under_cursor = self.unresolved_wikilink_under_cursor().is_some();
        crate::context_menu_surfaces::EditorBodyAvailability {
            symbol_under_cursor,
            quick_fix_available,
            has_selection,
            definition_available,
            unresolved_link_under_cursor,
        }
    }

    /// MT-070/MT-057: the syntactic `[[title]]` wikilink under the primary caret, or `None`. This helper
    /// only scans the caret's line for a `[[…]]` span covering the caret byte; create-note availability
    /// must use [`Self::unresolved_wikilink_under_cursor`], which also requires an authoritative resolver
    /// snapshot. The inner title is returned trimmed; an empty `[[]]` yields `None`.
    pub fn wikilink_under_cursor(&self) -> Option<String> {
        let offset = self.primary_cursor_offset();
        self.with_buffer(|b| {
            let (line, _) = byte_to_line_col(offset, b);
            let start = b.line_to_byte(line)?;
            let end = b.line_to_byte(line + 1).unwrap_or_else(|| b.len_bytes());
            let text = b.byte_slice_to_string(start..end);
            // The caret's BYTE offset within the line (byte units so non-ASCII lines never mis-slice).
            let col = offset.saturating_sub(start).min(text.len());
            let mut i = 0usize;
            while let Some(open_rel) = text[i..].find("[[") {
                let open = i + open_rel;
                let close = match text[open + 2..].find("]]") {
                    Some(c) => open + 2 + c,
                    None => break,
                };
                // The span covers the caret when it sits anywhere over `[[title]]` (inclusive edges).
                if col >= open && col <= close + 2 {
                    let title = text[open + 2..close].trim();
                    if title.is_empty() {
                        return None;
                    }
                    return Some(title.to_owned());
                }
                i = close + 2;
            }
            None
        })
    }

    /// Replace the code pane's resolver snapshot. The shell passes `None` until the mounted rich
    /// runtime has a successful workspace seed, preventing duplicate note creation during load/error.
    pub fn set_wikilink_resolver_index(
        &self,
        index: Option<crate::rich_editor::wikilinks::resolver::ResolverIndex>,
    ) {
        *self
            .wikilink_resolver_index
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = index;
    }

    /// The link under the cursor only when the authoritative resolver confirms it has no target.
    pub fn unresolved_wikilink_under_cursor(&self) -> Option<String> {
        let title = self.wikilink_under_cursor()?;
        let index = self
            .wikilink_resolver_index
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let index = index.as_ref()?;
        match crate::rich_editor::wikilinks::resolver::resolve_wikilink(index, &title) {
            crate::rich_editor::wikilinks::resolver::WikilinkResolution::Unresolved { .. } => {
                Some(title)
            }
            crate::rich_editor::wikilinks::resolver::WikilinkResolution::Resolved { .. }
            | crate::rich_editor::wikilinks::resolver::WikilinkResolution::Ambiguous { .. } => None,
        }
    }

    /// MT-046: build the `[[code:…]]` note-reference string for the current selection / identifier —
    /// the code -> note interconnection payload. The ref value follows the MT-034 `path#Symbol` shape
    /// the wikilink parser + cross-ref resolver consume: `[[code:{file_path}#{anchor}]]`. A selection
    /// must equal one tree-sitter identifier range; a bare caret resolves the identifier node under it.
    /// Returns `None` when the buffer has no parser-encodable file path or no exact identifier, so the
    /// menu entry disables rather than emitting a noncanonical or lossy reference.
    pub fn note_reference_for_cursor(&self) -> Option<String> {
        self.ensure_highlight_cache();
        let selection = self.selected_primary_text();
        let probe_byte = selection
            .as_ref()
            .map(|(start, _, _)| *start)
            .unwrap_or_else(|| self.primary_cursor_offset());
        let identifier_range = {
            let highlighter = self.highlighter.lock().unwrap_or_else(|e| e.into_inner());
            highlighter
                .as_ref()
                .and_then(|hl| hl.tree())
                .and_then(|tree| rename::identifier_range_at(tree, probe_byte))
        }?;
        if selection
            .as_ref()
            .is_some_and(|(start, end, _)| identifier_range != (*start..*end))
        {
            return None;
        }
        let anchor = self.with_buffer(|buffer| {
            (identifier_range.end <= buffer.len_bytes())
                .then(|| buffer.byte_slice_to_string(identifier_range.clone()))
        })?;
        let path = self.file_path();
        let path = path.as_str();
        crate::interop::cross_ref::format_code_note_reference(path, &anchor)
    }

    /// MT-046: the REAL 'Copy as note reference' command — build the `[[code:…]]` ref from the live
    /// selection/identifier ([`note_reference_for_cursor`](Self::note_reference_for_cursor)) and stage
    /// it for the factory render's SHARED-InteractionBus clipboard write
    /// (`interop_adapter::copy_note_reference_to_bus`). One path for the context-menu entry AND the
    /// `CodeEditorAction::CopyAsNoteReference` command dispatch. Returns `true` when a ref was staged.
    pub fn copy_as_note_reference(&self) -> bool {
        match self.note_reference_for_cursor() {
            Some(reference) => {
                *self
                    .pending_copy_note_reference
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(reference);
                true
            }
            None => false,
        }
    }

    /// MT-046: drain the staged `[[code:…]]` note reference (the factory render writes it to the shared
    /// bus clipboard; a test drains it to drive the real bus write). `None` when nothing is staged.
    pub fn take_pending_copy_note_reference(&self) -> Option<String> {
        self.pending_copy_note_reference
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// MT-070/MT-057: the 'Create note from link' handler — stage the `[[title]]` under the caret as
    /// the typed create-note intent the host drains ([`take_pending_create_note_link`]). A no-op when
    /// no wikilink is under the caret (the entry is disabled then, so this is belt-and-braces).
    fn stage_create_note_from_link(&self) {
        if let Some(title) = self.unresolved_wikilink_under_cursor() {
            *self
                .pending_create_note_link
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(title);
        }
    }

    /// MT-070/MT-057: drain the staged create-note-from-link intent (the `[[title]]` the confirmed menu
    /// entry captured). The host/shell routes it to the MT-057 create-note handler; a test drains it to
    /// prove the entry fired its REAL handler. `None` when nothing is staged.
    pub fn take_pending_create_note_link(&self) -> Option<String> {
        self.pending_create_note_link
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }

    /// Render the MT-006 outline (symbol) tree in the left side panel, with the AccessKit `Role::Tree`
    /// node `code_editor_outline` (AC-004 / HBR-SWARM). Each symbol row is a clickable
    /// `CollapsingHeader`-style entry; clicking it calls [`navigate_to_line`](Self::navigate_to_line)
    /// (fold-aware) to scroll the editor + move the caret to the symbol's line. The list scrolls
    /// (an outline can be long — MT step "use ScrollArea for the outline").
    fn render_outline_panel(&self, ui: &mut egui::Ui, syntax: &HsSyntaxTokens) {
        ui.scope_builder(
            egui::UiBuilder::new().id_salt(self.outline_panel_scope_id()),
            |ui| {
                let outline_node_id = ui.unique_id();
                ui.label(egui::RichText::new("OUTLINE").color(syntax.comment).small());
                ui.separator();

                let items = self.outline_items();
                let mut navigate_to: Option<usize> = None;
                // A source file can expose thousands of top-level symbols (generated bindings and large
                // flat modules are common). Building every outline row on the first editor frame defeats
                // the editor body's virtualization even though the text viewport itself is bounded. Use
                // egui's row virtualization so layout, labels, and AccessKit nodes are created only for
                // the visible outline window.
                let outline_row_height = ui
                    .text_style_height(&egui::TextStyle::Monospace)
                    .max(ui.spacing().interact_size.y);
                egui::ScrollArea::vertical()
                    .id_salt(("code-editor-outline-scroll", self.outline_panel_scope_id()))
                    .auto_shrink([false, false])
                    .show_rows(
                        ui,
                        outline_row_height,
                        items.len().max(1),
                        |ui, row_range| {
                            if items.is_empty() {
                                ui.label(
                                    egui::RichText::new("No symbols")
                                        .italics()
                                        .color(syntax.comment),
                                );
                                return;
                            }
                            for idx in row_range {
                                let item = &items[idx];
                                // Indent the row by the outline depth (MT step 2). A leading kind tag + the name.
                                let label = format!(
                                    "{}{} {}",
                                    "  ".repeat(item.indent),
                                    item.kind.label(),
                                    item.name
                                );
                                let resp = ui.add(
                                    egui::Label::new(egui::RichText::new(label).monospace())
                                        .sense(egui::Sense::click()),
                                );
                                let row_author = self.suffixed(&format!(
                                    "{CODE_EDITOR_OUTLINE_ROW_AUTHOR_PREFIX}{idx}"
                                ));
                                let row_label = format!("{} {}", item.kind.label(), item.name);
                                ui.ctx().accesskit_node_builder(resp.id, move |node| {
                                    node.set_role(accesskit::Role::TreeItem);
                                    node.set_author_id(row_author.clone());
                                    node.set_label(row_label.clone());
                                    node.add_action(accesskit::Action::Click);
                                });
                                if resp.clicked() {
                                    navigate_to = Some(item.line);
                                }
                                resp.on_hover_text(format!(
                                    "Go to line {} ({})",
                                    item.line + 1,
                                    item.kind.label()
                                ));
                                // Stable per-row egui id so the row is individually addressable; the row index is
                                // unique per frame (outline order is deterministic). (The container Tree node is
                                // the AC-004 addressable surface; individual rows live in egui's hashed id space,
                                // the same dynamic-row pattern the shell tree/list containers use.)
                                let _ = idx;
                            }
                        },
                    );

                // Emit the outline Tree node onto this scope's Ui id (AC-004 / HBR-SWARM).
                let author = self.outline_author_id();
                let count = items.len();
                ui.ctx()
                    .accesskit_node_builder(outline_node_id, move |node| {
                        node.set_role(accesskit::Role::Tree);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor outline".to_owned());
                        node.set_value(format!("{count} symbols"));
                    });

                // Navigate AFTER the borrow on `items` is released (fold-aware scroll + caret move).
                if let Some(line) = navigate_to {
                    // MT-052 jump-history record site #3 (outline / in-file symbol jump): record the pre-jump
                    // caret location so Navigate Back returns here, before the caret moves to the symbol line.
                    self.record_jump_origin();
                    self.navigate_to_line(line);
                }
            },
        );
    }

    /// Render the MT-006 minimap in the right side panel, with the AccessKit `Role::ScrollBar` node
    /// `code_editor_minimap` (AC-004 / HBR-SWARM). A click on the minimap scrolls the editor to the
    /// clicked line through the fold-aware mapping. `visible_buffer_range` is the editor viewport in
    /// BUFFER-line space (the indicator rect).
    fn render_minimap_panel(
        &self,
        ui: &mut egui::Ui,
        visible_buffer_range: std::ops::Range<usize>,
        total_lines: usize,
    ) {
        ui.scope_builder(
            egui::UiBuilder::new().id_salt(self.minimap_panel_scope_id()),
            |ui| {
                // Resolve this frame's minimap row layout (how many rows, at what compression). The
                // O(spans) color computation is CACHED keyed by (buffer_version, painted_rows, dark_mode)
                // so it runs only on an edit / resize / theme flip — the per-frame render is O(painted_rows)
                // (MT-002 frame-budget protection on a 100k-line file).
                let panel_height = ui.available_height().max(1.0);
                let ratio = Minimap::compression_ratio(total_lines, panel_height);
                let painted_rows = total_lines.div_ceil(ratio).max(1);
                let dark_mode = ui.visuals().dark_mode;
                let version = self.buffer_version.load(Ordering::Relaxed);
                let row_colors = self.minimap_row_colors(painted_rows, ratio, dark_mode, version);

                let response =
                    self.minimap
                        .render(ui, &row_colors, visible_buffer_range.clone(), total_lines);
                // Store the minimap's TRUE content rect (exactly the configured width) for the AC-006
                // midpoint-click geometry + AC-003 width assertion — the enclosing SidePanel adds frame
                // margins around this, so the panel's outer rect is wider.
                *self
                    .last_minimap_rect
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(response.content_rect);
                // A minimap click is a scroll-to request, routed through the fold-aware mapping (MT
                // positioning note) so a click lands on the correct row even with folds active.
                if let Some(line) = response.clicked_buffer_line {
                    let visible_line = self.buffer_line_to_visible_line(line);
                    self.scroll_to_line(visible_line);
                }

                // Emit the minimap ScrollBar node onto the REAL click/drag response id (AC-004 /
                // HBR-SWARM). It MUST carry an author_id — a
                // ScrollBar is an INTERACTIVE role the MT-025 accessibility gate flags if unnamed.
                let author = self.minimap_author_id();
                let value = format!(
                    "lines {}-{} of {total_lines}",
                    visible_buffer_range.start, visible_buffer_range.end
                );
                ui.ctx()
                    .accesskit_node_builder(response.response_id, move |node| {
                        node.set_role(accesskit::Role::ScrollBar);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor minimap".to_owned());
                        node.set_value(value.clone());
                    });
            },
        );
    }

    /// Render the MT-006 go-to-line palette as a small centered modal `egui::Window` (Ctrl+G). The
    /// single-line input is pre-populated with the current cursor line; Enter (or the Go button)
    /// submits, Escape closes. The AccessKit `Role::TextInput` node `code_editor_goto_line` is emitted
    /// so a swarm agent can address the input (AC-005 / HBR-SWARM). A no-op (and no node) when the
    /// palette is closed.
    fn render_goto_line_modal(&self, ui: &mut egui::Ui, syntax: &HsSyntaxTokens) {
        let Some(mut state) = self.goto_line_state() else {
            return;
        };
        let total_lines = self.with_buffer(|b| b.len_lines());
        let mut submit = false;
        let mut input_changed = false;

        let window_id = if self.instance.is_empty() {
            egui::Id::new("code_editor_goto_line_window")
        } else {
            egui::Id::new(format!("code_editor_goto_line_window#{}", self.instance))
        };

        egui::Window::new("Go to Line")
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 48.0))
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut state.input)
                            .id_salt(("code-editor-goto-line-input", self.text_id()))
                            .desired_width(120.0)
                            .hint_text(format!("Line 1-{total_lines}")),
                    );
                    // Auto-focus the input on open so typing goes straight to it.
                    if resp.changed() {
                        input_changed = true;
                    }
                    resp.request_focus();
                    if ui.button("Go").clicked() {
                        submit = true;
                    }
                });
                // Validity feedback: show the resolved 1-based target line, or an error for bad input.
                match state.parsed {
                    Some(line) => {
                        ui.label(
                            egui::RichText::new(format!("\u{2192} line {}", line + 1))
                                .color(syntax.comment)
                                .small(),
                        );
                    }
                    None if !state.input.trim().is_empty() => {
                        ui.label(
                            egui::RichText::new("not a line number")
                                .color(syntax.string)
                                .small(),
                        );
                    }
                    None => {}
                }
            });

        // Push the edited input back into the owned state (re-parses validity) so the next frame's modal
        // + a submit see the current value.
        if input_changed {
            self.set_goto_line_input(state.input.clone());
        }

        // Submit / close are handled by the Ctrl+G keymap too (process_cursor_input), but the Go button
        // path is handled here.
        if submit {
            self.submit_goto_line();
        }

        // Emit the go-to-line TextInput node (AC-005 / HBR-SWARM). Fixed id band (default panel) keeps
        // the NodeId stable; instances hash the suffixed author_id (RISK-004).
        let author = self.goto_line_author_id();
        let node_id = if self.instance.is_empty() {
            // SAFETY: a single hand-assigned fixed id in the disjoint nav band (372); never reused.
            unsafe { egui::Id::from_high_entropy_bits(PANEL_GOTO_LINE_NODE_ID) }
        } else {
            egui::Id::new(self.goto_line_author_id())
        };
        ui.ctx().accesskit_node_builder(node_id, move |node| {
            node.set_role(accesskit::Role::TextInput);
            node.set_author_id(author.clone());
            node.set_label("Code editor go to line".to_owned());
        });
    }

    // ── MT-053 author_id helpers + render ──────────────────────────────────────────────────────────

    /// The stable AccessKit author_id for the symbol-palette list container, instance-suffixed.
    pub fn symbol_palette_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_SYMBOL_PALETTE_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for the symbol-palette search input, instance-suffixed.
    pub fn symbol_palette_search_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_SYMBOL_PALETTE_SEARCH_AUTHOR_ID)
    }

    /// The stable AccessKit author_id for the sticky-scroll band container, instance-suffixed.
    pub fn sticky_scroll_author_id(&self) -> String {
        self.suffixed(CODE_EDITOR_STICKY_SCROLL_AUTHOR_ID)
    }

    /// Render the MT-053 in-file Go to Symbol palette as a centered modal `egui::Window` (Ctrl+Shift+O),
    /// mirroring the go-to-line modal + the MT-016/MT-017 overlay-modal pattern. A single-line fuzzy
    /// search input at the top, a scrollable result list below. Arrow keys move the selection (handled in
    /// `resolve_contextual`), Enter confirms + jumps, Escape closes; a row click also confirms that row.
    /// Emits the `code_editor_symbol_palette` (Role::List) container, the `code_editor_symbol_palette_search`
    /// (Role::TextInput) input, and a `symbol-{index}` (Role::ListItem) node per visible row (AC-003 /
    /// AC-005 / MC-005). A no-op (and no node) when the palette is closed.
    fn render_symbol_palette_modal(&self, ui: &mut egui::Ui, syntax: &HsSyntaxTokens) {
        if !self.is_symbol_palette_open() {
            return;
        }
        // Snapshot the current query + filtered rows + selection for this frame's render.
        let (mut query, results, selected) = {
            let palette = self
                .symbol_palette
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            (
                palette.query().to_owned(),
                palette.results().to_vec(),
                palette.selected_index(),
            )
        };
        let mut query_changed = false;
        let mut confirm_index: Option<usize> = None;

        let window_id = if self.instance.is_empty() {
            egui::Id::new("code_editor_symbol_palette_window")
        } else {
            egui::Id::new(format!(
                "code_editor_symbol_palette_window#{}",
                self.instance
            ))
        };

        let dialog_node_id = if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_SYMBOL_PALETTE_DIALOG_NODE_ID) }
        } else {
            egui::Id::new(format!("{}#dialog", self.symbol_palette_author_id()))
        };
        let list_node_id = if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_SYMBOL_PALETTE_LIST_NODE_ID) }
        } else {
            egui::Id::new(self.symbol_palette_author_id())
        };
        let search_node_id = if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_SYMBOL_PALETTE_SEARCH_NODE_ID) }
        } else {
            egui::Id::new(self.symbol_palette_search_author_id())
        };

        let search_author = self.symbol_palette_search_author_id();
        let list_author = self.symbol_palette_author_id();
        let row_count = results.len();

        egui::Window::new("Go to Symbol in File")
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_TOP, egui::vec2(0.0, 48.0))
            .show(ui.ctx(), |ui| {
                ui.set_min_width(360.0);
                // Search input.
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut query)
                        .id_salt(("code-editor-symbol-palette-input", self.text_id()))
                        .desired_width(340.0)
                        .hint_text("Go to symbol… (fuzzy)"),
                );
                if resp.changed() {
                    query_changed = true;
                }
                resp.request_focus();
                // Name the search node (Role::TextInput) so a swarm agent can address it.
                {
                    let author = search_author.clone();
                    ui.ctx()
                        .accesskit_node_builder(search_node_id, move |node| {
                            node.set_role(accesskit::Role::TextInput);
                            node.set_author_id(author.clone());
                            node.set_label("Code editor symbol palette search".to_owned());
                        });
                }

                ui.separator();

                // Result list (scrollable). Each row is clickable + carries a symbol-{index} ListItem node.
                egui::ScrollArea::vertical()
                    .id_salt(("code-editor-symbol-palette-scroll", self.text_id()))
                    .max_height(280.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if results.is_empty() {
                            ui.label(
                                egui::RichText::new("No matching symbols")
                                    .italics()
                                    .color(syntax.comment),
                            );
                        }
                        let default_text = ui.visuals().text_color();
                        for (idx, sym) in results.iter().enumerate() {
                            let is_sel = idx == selected;
                            let row_resp = ui.add(
                                egui::Label::new(
                                    egui::RichText::new(sym.display_label())
                                        .monospace()
                                        .color(if is_sel { syntax.keyword } else { default_text }),
                                )
                                .sense(egui::Sense::click()),
                            );
                            if is_sel {
                                // Faint highlight on the selected row (a UI affordance, not a syntax token).
                                ui.painter().rect_filled(
                                    row_resp.rect,
                                    2.0,
                                    egui::Color32::from_rgba_premultiplied(80, 110, 170, 40),
                                );
                            }
                            if row_resp.clicked() {
                                confirm_index = Some(idx);
                            }
                            // Per-row ListItem node (capped — RISK / node budget). Addressable by
                            // symbol-{index} so a swarm agent can click a result by id.
                            if idx < MAX_ACCESSKIT_SYMBOL_ROWS {
                                let author = format!("{CODE_EDITOR_SYMBOL_ROW_AUTHOR_PREFIX}{idx}");
                                let author = self.suffixed(&author);
                                let label = sym.display_label();
                                ui.ctx().accesskit_node_builder(row_resp.id, move |node| {
                                    node.set_role(accesskit::Role::ListItem);
                                    node.set_author_id(author.clone());
                                    node.set_label(label.clone());
                                    node.add_action(accesskit::Action::Click);
                                });
                            }
                        }
                    });

                // The list CONTAINER node (Role::List, AC-003 — the node the test asserts Ctrl+Shift+O
                // produced). Emitted onto a fixed id so it is stable across frames.
                let author = list_author.clone();
                let value = format!("{row_count} symbols");
                ui.ctx().accesskit_node_builder(list_node_id, move |node| {
                    node.set_role(accesskit::Role::List);
                    node.set_author_id(author.clone());
                    node.set_label("Code editor symbol palette".to_owned());
                    node.set_value(value.clone());
                });
            });

        // The dialog root node (Role::Dialog, modal) so the overlay is addressable as a unit (the same
        // pattern the MT-016 command palette / MT-017 switcher use). A Dialog is non-interactive, so it
        // does not need the interactive-gate author_id, but we set one for symmetry + discoverability.
        {
            let author = format!("{}_dialog", self.symbol_palette_author_id());
            ui.ctx()
                .accesskit_node_builder(dialog_node_id, move |node| {
                    node.set_role(accesskit::Role::Dialog);
                    node.set_author_id(author.clone());
                    node.set_modal();
                    node.set_label("Go to symbol in file".to_owned());
                });
        }

        // Push the edited query back into the owned state (re-filters) so the next frame + a confirm see
        // the current value.
        if query_changed {
            self.set_symbol_palette_query(query);
        }
        // A row click confirms that exact row: set the selection to it, then confirm.
        if let Some(idx) = confirm_index {
            {
                let mut palette = self
                    .symbol_palette
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                // Re-derive the selection to the clicked row by stepping (clamped) — simplest correct path
                // without exposing a set_selected.
                let cur = palette.selected_index();
                if idx >= cur {
                    for _ in 0..(idx - cur) {
                        palette.select_next();
                    }
                } else {
                    for _ in 0..(cur - idx) {
                        palette.select_prev();
                    }
                }
            }
            self.confirm_symbol_palette();
        }
    }

    /// Render the MT-053 sticky-scroll band: a pinned top strip of the center editor area showing the
    /// declaration lines of every scope enclosing the first visible line, outermost-first, capped at
    /// `max_sticky_lines`. Reserves vertical space = `headers.len() * line_height` by claiming a
    /// `TopBottomPanel::top` so the scroll area below gets the remaining height (the first scrolled line is
    /// NEVER occluded — RISK-003 / MC-003, structural reservation). Clicking a header scrolls to its scope
    /// (the SAME fold-aware scroll path). Emits the `code_editor_sticky_scroll` (Role::GenericContainer)
    /// container node and a `sticky-header-{depth}` (Role::Button) node per header. A no-op (and no nodes)
    /// when no scope encloses the viewport top.
    fn render_sticky_band(&self, ui: &mut egui::Ui, total_lines: usize, line_height: f32) {
        // MT-035: honor the Settings sticky-scroll toggle — when disabled, emit no band + no headers.
        if !self.sticky_scroll_enabled() {
            return;
        }
        // Recompute headers EVERY frame from the CURRENT scroll offset + the live fold regions (RISK-004 /
        // MC-004 — no caching across edits). The first visible BUFFER line is the start of the last painted
        // buffer-line window (`show` captured it last frame; on the first frame it is 0..0 -> top line 0).
        let viewport_top = self.last_painted_buffer_range(total_lines).start;
        let fold_set = self.fold_set();
        let headers = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            self.sticky_scroll
                .compute(viewport_top, &fold_set.regions, &buffer)
        };
        if headers.is_empty() {
            return;
        }

        let band_height = headers.len() as f32 * line_height;
        let panel_id = if self.instance.is_empty() {
            egui::Id::new("code_editor_sticky_scroll_panel")
        } else {
            egui::Id::new(format!("code_editor_sticky_scroll_panel#{}", self.instance))
        };

        let syntax = syntax_tokens_for(ui.visuals());
        let mut click_line: Option<usize> = None;

        egui::TopBottomPanel::top(panel_id)
            .resizable(false)
            .exact_height(band_height)
            .show_separator_line(true)
            .show_inside(ui, |ui| {
                // Paint the band background from the gutter/editor background so it reads as chrome.
                let band_rect = ui.available_rect_before_wrap();
                if ui.is_rect_visible(band_rect) {
                    ui.painter().rect_filled(band_rect, 0.0, syntax.background);
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                let header_text_color = ui.visuals().text_color();
                for header in &headers {
                    // Indent by depth so the pinned stack reads like the source nesting.
                    let text = format!("{}{}", "  ".repeat(header.depth), header.text.trim_start());
                    let resp = ui
                        .add(
                            egui::Label::new(
                                egui::RichText::new(text)
                                    .monospace()
                                    .color(header_text_color),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .on_hover_text(format!("Scroll to line {}", header.line + 1));
                    let resp_id = resp.id;
                    if resp.clicked() {
                        click_line = Some(header.line);
                    }
                    // Per-header Button node (Role::Button), addressable by sticky-header-{depth} so a
                    // swarm agent can click a header to scroll to its scope (AC-006 / MC-005).
                    let author = self.suffixed(&format!(
                        "{CODE_EDITOR_STICKY_HEADER_AUTHOR_PREFIX}{}",
                        header.depth
                    ));
                    let label = format!("Sticky header: {}", header.text.trim());
                    ui.ctx().accesskit_node_builder(resp_id, move |node| {
                        node.set_role(accesskit::Role::Button);
                        node.set_author_id(author.clone());
                        node.set_label(label.clone());
                        node.add_action(accesskit::Action::Click);
                    });
                }

                // The band CONTAINER node (Role::GenericContainer, AC-004). Emitted onto a fixed id so it
                // is stable across frames.
                let container_node_id = if self.instance.is_empty() {
                    unsafe { egui::Id::from_high_entropy_bits(PANEL_STICKY_SCROLL_NODE_ID) }
                } else {
                    egui::Id::new(self.sticky_scroll_author_id())
                };
                let author = self.sticky_scroll_author_id();
                let count = headers.len();
                ui.ctx()
                    .accesskit_node_builder(container_node_id, move |node| {
                        node.set_role(accesskit::Role::GenericContainer);
                        node.set_author_id(author.clone());
                        node.set_label("Code editor sticky scroll".to_owned());
                        node.set_value(format!("{count} pinned headers"));
                    });
            });

        // Apply a header click AFTER the panel closure (fold-aware scroll, the same path JumpTo uses).
        if let Some(line) = click_line {
            self.record_jump_origin();
            let visible_line = self.buffer_line_to_visible_line(line);
            self.scroll_to_line(visible_line);
        }
    }

    /// The `egui::Id` salt for the outline panel scope (default uses the fixed nav-band slot; instances
    /// hash the suffixed author_id so two panels never share an id — RISK-004).
    fn outline_panel_scope_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_OUTLINE_NODE_ID) }
        } else {
            egui::Id::new(self.outline_author_id())
        }
    }

    /// The `egui::Id` salt for the minimap panel scope (default uses the fixed nav-band slot; instances
    /// hash the suffixed author_id — RISK-004).
    fn minimap_panel_scope_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_MINIMAP_NODE_ID) }
        } else {
            egui::Id::new(self.minimap_author_id())
        }
    }

    /// The editor's most-recent painted row window expressed in BUFFER-line space (the minimap viewport
    /// indicator). The panel captures `last_visible_range` in VISIBLE-line space (post-fold); map both
    /// ends back to buffer lines through the fold set. Before the first render this is `0..0`.
    fn last_painted_buffer_range(&self, total_lines: usize) -> std::ops::Range<usize> {
        let visible = self.last_visible_range();
        if visible.is_empty() {
            return 0..0;
        }
        // `show` rebuilt the fold visible-map against the live line count earlier THIS frame, so the map
        // is already current — just look up the two ends (no extra O(total_lines) rebuild here, which
        // would double the per-frame fold-map cost on a 100k-line file — MT-002 frame budget).
        let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
        let start = set.visible_line_to_buffer_line(visible.start);
        let end = set.visible_line_to_buffer_line(visible.end.saturating_sub(1)) + 1;
        start..end.min(total_lines)
    }

    /// Render the MT-004 find bar pinned to the top-right of `panel_rect`, when the bar is open. The
    /// bar is a themed `egui::Frame` containing: the find input (a single-line `TextEdit`), the
    /// case/whole-word/regex toggle buttons, Prev/Next buttons, a `N of M` match counter, and — in
    /// replace mode (Ctrl+H) — a second `TextEdit` for the replacement plus Replace / Replace-All
    /// buttons. Each widget's text/toggle change is pushed back into `find_state` and triggers a
    /// re-search (so typing finds incrementally). The stable AccessKit author_id nodes
    /// (`code_editor_find_bar` / `code_editor_replace_bar` / `code_editor_find_next` /
    /// `code_editor_find_prev`) are emitted afterward so a swarm agent can address each control (AC-004
    /// / HBR-SWARM). A no-op (and no nodes) when the bar is closed (AC-006).
    fn render_find_bar(&self, ui: &mut egui::Ui, panel_rect: egui::Rect, syntax: &HsSyntaxTokens) {
        // Snapshot the current state; bail (and emit no nodes) when closed.
        let Some(mut state) = self.find_state() else {
            self.find_text_input_focused.store(false, Ordering::Release);
            *self
                .live_find_node_id
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            return;
        };

        // Pin to the top-right corner of the editor area (VS Code style — a floating widget, not a side
        // panel — MT step 6). Width 400 px, height grows with the replace row.
        let bar_width = 400.0_f32.min(panel_rect.width().max(120.0));
        let bar_height = if state.show_replace {
            FIND_BAR_HEIGHT_REPLACE_PX
        } else {
            FIND_BAR_HEIGHT_SINGLE_PX
        };
        let bar_min = egui::pos2(
            panel_rect.right() - bar_width - 4.0,
            panel_rect.top() + FIND_BAR_TOP_MARGIN_PX,
        );
        let bar_rect = egui::Rect::from_min_size(bar_min, egui::vec2(bar_width, bar_height));

        let mut query_changed = false;
        let mut close_requested = false;
        let mut text_input_focused = false;

        let frame = egui::Frame::popup(ui.style()).fill(syntax.background);
        // `ui.put` would force a fixed size onto a single widget; for a multi-widget bar use a child UI
        // constrained to `bar_rect` so the frame + controls lay out inside the pinned rectangle.
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(bar_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        frame.show(&mut child, |ui| {
            // FIND row: input + toggles + prev/next + counter.
            ui.horizontal(|ui| {
                let find_resp = ui.add(
                    egui::TextEdit::singleline(&mut state.query.pattern)
                        .id_salt(("code-editor-find-input", self.text_id()))
                        .desired_width(150.0)
                        .hint_text("Find"),
                );
                // MT-108 (MT-004 residual): auto-focus the find input on the first frame after the bar
                // opens (VS Code parity), then swap-clear so focus is not re-stolen every frame.
                if self.find_focus_pending.swap(false, Ordering::AcqRel) {
                    find_resp.request_focus();
                }
                text_input_focused |= find_resp.has_focus();
                if find_resp.changed() {
                    query_changed = true;
                }

                // Case / whole-word / regex toggles (selectable_label so the on-state is visible).
                if ui
                    .selectable_label(state.query.case_sensitive, "Aa")
                    .on_hover_text("Match case")
                    .clicked()
                {
                    state.query.case_sensitive = !state.query.case_sensitive;
                    query_changed = true;
                }
                if ui
                    .selectable_label(state.query.whole_word, "\u{2423}W")
                    .on_hover_text("Whole word")
                    .clicked()
                {
                    state.query.whole_word = !state.query.whole_word;
                    query_changed = true;
                }
                if ui
                    .selectable_label(state.query.is_regex, ".*")
                    .on_hover_text("Use regular expression")
                    .clicked()
                {
                    state.query.is_regex = !state.query.is_regex;
                    query_changed = true;
                }

                if ui
                    .button("\u{2191}")
                    .on_hover_text("Previous match")
                    .clicked()
                {
                    self.prev_match();
                }
                if ui.button("\u{2193}").on_hover_text("Next match").clicked() {
                    self.next_match();
                }
                ui.label(
                    self.find_state()
                        .map(|s| s.counter_label())
                        .unwrap_or_default(),
                );
                if ui.button("\u{2715}").on_hover_text("Close (Esc)").clicked() {
                    close_requested = true;
                }
            });
            // The regex compile error, if any (AC-003: surfaced, never a panic).
            if !state.error.is_empty() {
                ui.colored_label(syntax.string, format!("regex error: {}", state.error));
            }
            // REPLACE row (Ctrl+H only).
            if state.show_replace {
                ui.horizontal(|ui| {
                    let replace_resp = ui.add(
                        egui::TextEdit::singleline(&mut state.replace_text)
                            .id_salt(("code-editor-replace-input", self.text_id()))
                            .desired_width(150.0)
                            .hint_text("Replace"),
                    );
                    text_input_focused |= replace_resp.has_focus();
                    if ui.button("Replace").clicked() {
                        self.set_replace_text(state.replace_text.clone());
                        self.replace_current();
                    }
                    if ui.button("Replace All").clicked() {
                        self.set_replace_text(state.replace_text.clone());
                        self.replace_all();
                    }
                });
                // MT-108 (MT-004 residual): capped Replace All progress hint. Read fresh so it reflects
                // the click that just ran this frame.
                if let Some(remaining) = self
                    .find_state()
                    .map(|s| s.replace_all_remaining)
                    .filter(|r| *r > 0)
                {
                    ui.colored_label(
                        syntax.string,
                        format!("{remaining} more not yet replaced — click Replace All again"),
                    );
                }
            }
        });
        self.find_text_input_focused
            .store(text_input_focused, Ordering::Release);

        // Push the edited query / replace text back into the owned state and re-search if needed. We do
        // this AFTER the frame closes so the borrow on `state` is released. The replace text is pushed
        // unconditionally (cheap) so a keystroke in the replace input is not lost before a button click.
        self.set_replace_text(state.replace_text.clone());
        if query_changed {
            self.set_find_query(state.query.pattern.clone());
            self.set_find_toggles(
                state.query.case_sensitive,
                state.query.whole_word,
                state.query.is_regex,
            );
        }
        if close_requested {
            self.close_find();
            return; // closed -> emit no find-bar nodes this frame (AC-006)
        }

        // Emit the stable AccessKit author_id nodes for the find-bar controls (AC-004 / HBR-SWARM) onto
        // fixed ids in the find-bar band, as children of the container scope's Ui. (The MT contract
        // names `Role::SearchBox` for the find input, which does NOT exist in accesskit 0.21 —
        // `Role::SearchInput` is the field-correct search-input role; AC-004/PT-004 assert the
        // author_id, not the role string, so this satisfies the AC with the real API. Same deviation
        // discipline as the MT-003 TextCursor -> Caret fix.)
        self.emit_find_bar_nodes(ui);
    }

    /// Emit the four stable find-bar AccessKit nodes (`code_editor_find_bar` SearchInput,
    /// `code_editor_replace_bar` TextInput, `code_editor_find_next` / `code_editor_find_prev` Button) so
    /// a swarm agent can address each control by stable id (AC-004 / HBR-SWARM). The replace node is
    /// emitted only in replace mode. Fixed ids in the find-bar band (default panel) keep the NodeIds
    /// stable across frames; instances hash the suffixed author_id (RISK-004).
    fn emit_find_bar_nodes(&self, ui: &egui::Ui) {
        let show_replace = self
            .find_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|s| s.show_replace)
            .unwrap_or(false);
        let find_value = self
            .find_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|s| s.query.pattern.clone())
            .unwrap_or_default();

        let find_author = self.suffixed(CODE_EDITOR_FIND_BAR_AUTHOR_ID);
        let find_node_id =
            self.find_node_id(PANEL_FIND_BAR_NODE_ID, CODE_EDITOR_FIND_BAR_AUTHOR_ID);
        *self
            .live_find_node_id
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(find_node_id);
        ui.ctx().accesskit_node_builder(find_node_id, move |node| {
            // DEVIATION (API-correct): the contract names `Role::SearchBox`, which does not exist
            // in accesskit 0.21; `Role::SearchInput` is the field-correct search-input role.
            node.set_role(accesskit::Role::SearchInput);
            node.set_author_id(find_author.clone());
            node.set_label("Code editor find".to_owned());
            node.set_value(find_value.clone());
            node.add_action(accesskit::Action::SetValue);
        });

        let next_author = self.suffixed(CODE_EDITOR_FIND_NEXT_AUTHOR_ID);
        ui.ctx().accesskit_node_builder(
            self.find_node_id(PANEL_FIND_NEXT_NODE_ID, CODE_EDITOR_FIND_NEXT_AUTHOR_ID),
            move |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(next_author.clone());
                node.set_label("Find next".to_owned());
            },
        );

        let prev_author = self.suffixed(CODE_EDITOR_FIND_PREV_AUTHOR_ID);
        ui.ctx().accesskit_node_builder(
            self.find_node_id(PANEL_FIND_PREV_NODE_ID, CODE_EDITOR_FIND_PREV_AUTHOR_ID),
            move |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(prev_author.clone());
                node.set_label("Find previous".to_owned());
            },
        );

        if show_replace {
            let replace_author = self.suffixed(CODE_EDITOR_REPLACE_BAR_AUTHOR_ID);
            ui.ctx().accesskit_node_builder(
                self.find_node_id(PANEL_REPLACE_BAR_NODE_ID, CODE_EDITOR_REPLACE_BAR_AUTHOR_ID),
                move |node| {
                    node.set_role(accesskit::Role::TextInput);
                    node.set_author_id(replace_author.clone());
                    node.set_label("Code editor replace".to_owned());
                },
            );
        }
    }

    /// The fixed `egui::Id` for a find-bar node (default panel uses the find-bar band slot; an instance
    /// hashes the suffixed author_id so two panels never share a node id — RISK-004).
    fn find_node_id(&self, band_slot: u64, author_base: &str) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: each band slot is a distinct fixed id in the disjoint find-bar band, never reused.
            unsafe { egui::Id::from_high_entropy_bits(band_slot) }
        } else {
            egui::Id::new(self.suffixed(author_base))
        }
    }

    /// Measure + cache the monospace line height (px) used by the virtualizer, returning the cached
    /// value on subsequent frames (implementation note). The measured value is the glyph row height;
    /// `show_rows` adds item spacing itself, and the rows zero item-spacing, so this is the right
    /// row-height argument.
    fn line_height(&self, ui: &egui::Ui) -> f32 {
        let mut cached = self
            .line_height_px
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(h) = *cached {
            return h;
        }
        // MT-054 ROW-PITCH UNIT FIX: measure the row height of the EXACT live `mono_font()` galley
        // `render_line` paints glyphs with — NOT `text_style_height(Monospace)`,
        // which reads the style's (potentially differently sized) Monospace TextStyle. This is the ONE
        // row unit everywhere: the painted row pitch (each row's label galley is this tall and the rows
        // scope pins `interact_size.y` to it, so egui advances exactly this much per row), the
        // `show_rows` stride, the gutter row_top, the cursor/find/whitespace overlays, and every
        // decoration y. Deriving it from any other metric re-opens the ghost-bracket / gutter-drift
        // unit mismatch the Wave-B audit measured.
        let font = self.mono_font();
        // MT-035 wave-7: scale the measured natural row height by the LIVE line-height multiplier so lines
        // are spaced by the multiplier. The glyph galley still paints at the font's natural height; the
        // extra height is inter-line spacing (a >1.0 multiplier is a real, cache-invalidated respacing —
        // NOT a dead toggle). The single row unit (`line_height`) feeds the `show_rows` stride, the gutter,
        // the cursor/whitespace/decoration overlays, so multiplying HERE respaces everything consistently.
        let h = (ui.fonts_mut(|f| f.row_height(&font)) * self.line_height_multiplier()).max(1.0);
        *cached = Some(h);
        h
    }

    /// Measure + cache the monospace glyph advance width (px), measured with the EXACT
    /// live [`Self::mono_font`] that `render_line` paints glyphs with — so a caret at column
    /// `c` lands on column `c`'s glyph with no x-unit drift (MT-003 positioning requirement /
    /// implementation note 4). All monospace glyphs share one advance, so the space ' ' is
    /// representative. Falls back to half the line height if a font measurement is unavailable.
    fn glyph_width(&self, ui: &egui::Ui) -> f32 {
        let mut cached = self
            .glyph_width_px
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(w) = *cached {
            return w;
        }
        let font = self.mono_font();
        // `FontsView::glyph_width` takes `&mut self` (it lazily lays out the glyph), so use
        // `fonts_mut`. All monospace glyphs share one advance width, so ' ' is representative.
        let w = ui.fonts_mut(|f| f.glyph_width(&font, ' ')).max(1.0);
        *cached = Some(w);
        w
    }

    /// Render the rows for `row_range` (the virtualized visible window `show_rows` selected) and emit
    /// the inner `Role::TextInput` node. Split out so the text-area scope nests under the scroll-area
    /// scope (parent->child linkage for AC-005). The node is emitted onto this nested scope's own `Ui`
    /// id, which egui parents under the scroll scope's `Ui` node.
    #[allow(clippy::too_many_arguments)]
    fn render_rows(
        &self,
        ui: &mut egui::Ui,
        row_range: std::ops::Range<usize>,
        syntax: &HsSyntaxTokens,
        total_lines: usize,
        visible_lines: usize,
        text_id: egui::Id,
        text_author: &str,
        line_height: f32,
        glyph_width: f32,
    ) {
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt(text_id)
                .sense(egui::Sense::focusable_noninteractive()),
            |ui| {
                let text_node_id = ui.unique_id();
                // WP-KERNEL-012 MT-080: record the LIVE text-node egui id so `consume_swarm_text_actions` reads
                // swarm SetValue/ReplaceSelectedText requests at the EXACT node the tree emitted (AC-080-6).
                *self
                    .live_text_node_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(text_node_id);
                ui.style_mut().spacing.item_spacing.y = 0.0;
                // MT-054 ROW-PITCH UNIT FIX: `ui.horizontal` floors each row's frame at
                // `spacing.interact_size.y` (egui 0.33.3 ui.rs:2700 — 18.0 by default), while `show_rows`,
                // the gutter, the cursor overlay, and every decoration stride by `line_height` (~15.1 for
                // monospace 13). Pin the floor to EXACTLY `line_height` so each painted row advances egui's
                // cursor by exactly one `line_height` (the label galley is measured from the same
                // live `mono_font()` and is exactly this tall, so the centered content has
                // zero vertical offset). One unit everywhere — the Wave-B audit's ghost-bracket/gutter-drift
                // root cause.
                ui.style_mut().spacing.interact_size.y = line_height;

                // Capture the screen-space TOP-LEFT of the painted text rows BEFORE painting (the cursor
                // overlay + pointer hit-testing map (line,col) against this origin — MT-003). `cursor()` is
                // egui's next-widget position, i.e. the top-left of the first row about to be painted.
                let origin = ui.cursor().min;

                // `row_range` is in VISIBLE (post-fold) line space (MT-005). Clamp the upper bound to the
                // visible line count defensively (show_rows already clamps, but a stale range must never
                // index past the visible document).
                let visible_end = row_range.end.min(visible_lines);

                // Map the visible window to a BUFFER line window so the highlight-span clip + rendering use
                // real buffer coordinates. The first visible row maps to its buffer line; the last visible
                // row maps to its buffer line (its end is that buffer line + 1, but a folded region between
                // visible rows means the buffer window can be WIDER than the visible window — that is fine,
                // the per-row loop skips the hidden lines).
                let (first_buffer_line, last_buffer_line) = {
                    let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
                    let first = set.visible_line_to_buffer_line(row_range.start);
                    // The buffer line of the last painted visible row (inclusive), for the span byte window.
                    let last = if visible_end > row_range.start {
                        set.visible_line_to_buffer_line(visible_end - 1)
                    } else {
                        first
                    };
                    (first, last)
                };
                // Buffer-line exclusive end for the span byte window: one past the last folded region's end
                // if the last visible row is a folded region start, else last+1.
                let buffer_end = {
                    let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
                    match set.region_starting_at(last_buffer_line) {
                        Some(r) if r.folded => r.end_line + 1,
                        _ => last_buffer_line + 1,
                    }
                }
                .min(total_lines);

                // CLIP the highlight span list to the BUFFER byte window ONCE per frame (MT-002 step 3),
                // rather than scanning the whole span list per line. The cache is sorted by start byte, so a
                // binary search bounds the window to just the spans that can touch the painted rows.
                let (win_start, win_end) = self.with_buffer(|b| {
                    let ws = b.line_to_byte(first_buffer_line).unwrap_or(0);
                    let we = b.line_to_byte(buffer_end).unwrap_or_else(|| b.len_bytes());
                    (ws, we)
                });
                let visible_spans = self.spans_in_byte_window(win_start, win_end);

                // Paint one row per VISIBLE line index, mapping each to its buffer line (MT step 4). When the
                // buffer line is the start of a FOLDED region, render the collapsed summary label instead of
                // the line text; the hidden lines are simply never visited (they are not in the visible map).
                // MT-054/MT-005 FOLD-AWARE DECORATIONS: record the buffer line of EACH painted row, in row
                // order, so every decoration/overlay painter below maps a buffer line to its PAINTED row
                // offset through this list (a fold makes the buffer lines non-contiguous — `line -
                // first_line` is NOT the row offset). The list is strictly ascending (the fold map is
                // monotonic), so a binary search resolves a line to its row or `None` when hidden/off-window.
                let mut painted_lines: Vec<usize> =
                    Vec::with_capacity(visible_end - row_range.start);
                for visible_idx in row_range.start..visible_end {
                    let buffer_line = {
                        let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
                        set.visible_line_to_buffer_line(visible_idx)
                    };
                    painted_lines.push(buffer_line);
                    let folded_label = {
                        let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
                        match set.region_starting_at(buffer_line) {
                            Some(r) if r.folded => Some(r.label.clone()),
                            _ => None,
                        }
                    };
                    match folded_label {
                        Some(label) => self.render_fold_label_line(ui, &label, syntax),
                        None => self.render_line(ui, buffer_line, &visible_spans, syntax),
                    }
                }
                *self
                    .last_gutter_paint_rows
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = painted_lines
                    .iter()
                    .copied()
                    .map(GutterPaintRow::first_fragment)
                    .collect();

                // Store the painted-row geometry so `process_cursor_input` (pointer hit-testing) and the
                // overlay share egui's ACTUAL layout — no separate recompute (the MT-002 unit discipline:
                // sans-spacing line_height, the SAME glyph FontId). `first_line` is the BUFFER line of the
                // first painted row; the overlay maps a cursor's buffer (line,col) against it. NOTE: with a
                // folded region inside the window the buffer lines are non-contiguous, so the cursor overlay
                // (MT-003) positions correctly only for cursors on visible lines — a cursor on a hidden line
                // is simply not drawn (it is off the visible window), which is the correct behavior.
                let geometry = RowGeometry {
                    left: origin.x,
                    top: origin.y,
                    first_line: first_buffer_line,
                    line_height,
                };
                *self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()) = Some(geometry);

                // MT-054: paint the editor-chrome decorations (indent guides, bracket-pair colorization,
                // matching-bracket highlight) over the painted rows in the contract z-order: indent guides
                // first (faint lines that sit in the whitespace columns, below the glyphs), then re-draw each
                // bracket glyph in its depth color, then the matching-bracket highlight box. All theme-sourced
                // (CONTROL-4 — colors come from the palette tokens, never a hex literal here). Each visible
                // row in this non-wrap path is one logical line, so every row is a `wrap_index == 0` first
                // fragment and carries its indent guides (RISK-007 trivially holds when wrap is off).
                // FOLD-AWARE: `painted_lines` (the buffer line per painted row) is the ONLY line->row map —
                // hidden folded lines are not in it, so their decorations are never painted, and rows after
                // a fold land at their real painted offset (the Wave-B fold-mapping fix).
                self.paint_chrome_decorations(ui, &geometry, glyph_width, &painted_lines, None);

                // MT-071: when the render-whitespace toggle is ON, overlay middots for spaces + arrows for
                // tabs in the painted row window (VS Code's "render whitespace" — read from the doc-model
                // flag the status-bar segment flips). Theme-sourced color (no hex literal); restricted to the
                // visible window so it stays cheap on a large file. A no-op when the toggle is off (the
                // baseline render is unchanged).
                if self.render_whitespace() {
                    self.paint_whitespace_glyphs(
                        ui,
                        &geometry,
                        glyph_width,
                        &painted_lines,
                        syntax,
                        None,
                    );
                }

                // MT-004: paint the find-match highlights (below the carets) so a caret/selection stays
                // visible on top of a match rect. Restricted to the painted row window (the same sans-spacing
                // line_height + monospace glyph_width units as the cursor overlay), fold-aware via
                // `painted_lines`.
                self.paint_match_highlights(ui, &geometry, glyph_width, &painted_lines, None);

                // MT-003: paint every caret + selection as a painter overlay OVER the rows, restricted to
                // the painted row window so carets align exactly with rendered glyphs (no draw for cursors
                // scrolled off-screen, none for cursors on fold-hidden lines). Row y comes from the painted
                // row offset in `painted_lines` (fold-aware — the MT-003-era `y_for` contiguity fix) + the
                // same monospace glyph width.
                self.paint_cursor_overlay(ui, &geometry, glyph_width, &painted_lines, syntax, None);

                // Emit the TextInput node onto this nested scope's Ui id (AC-005). Because this scope is a
                // child of the scroll-area scope (itself a child of the container), the node is a
                // descendant of the container node.
                //
                // WP-KERNEL-012 MT-076 (AC7): while an IME composition is in progress, expose the in-progress
                // preedit text in the TextInput node's value so a screen reader / swarm agent can OBSERVE the
                // composition state (reuses the existing editable text node — no new tree). When idle the value
                // is the unchanged line count.
                let author = text_author.to_owned();
                let preedit = self
                    .preedit
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone();
                let value = if preedit.is_empty() {
                    format!("{total_lines} lines")
                } else {
                    format!("{total_lines} lines (composing: {preedit})")
                };
                ui.ctx().accesskit_node_builder(text_node_id, move |node| {
                    node.set_role(accesskit::Role::TextInput);
                    node.set_author_id(author.clone());
                    node.set_label("Code editor text".to_owned());
                    node.set_value(value.clone());
                    // WP-KERNEL-012 MT-080 (AC-080-6 / MT-043 swarm-authoring): advertise the two text-edit
                    // actions a swarm agent uses to AUTHOR code by id. `SetValue` replaces the WHOLE buffer
                    // (set_text); `ReplaceSelectedText` inserts at the selection/carets (insert_text, which
                    // replaces the active selection). Declaring them here makes the node's editable contract
                    // discoverable out-of-process; `consume_swarm_text_actions` drains the matching requests.
                    node.add_action(accesskit::Action::SetValue);
                    node.add_action(accesskit::Action::ReplaceSelectedText);
                    node.add_action(accesskit::Action::Focus);
                    // MT-041 swarm activation: clicking the stable text surface focuses the real
                    // editor, matching the pointer activation semantics and the editor.* action
                    // contract. `consume_swarm_text_actions` drains this request at the live node id.
                    node.add_action(accesskit::Action::Click);
                });

                // MT-003 AC-004: emit one `Role::Caret` AccessKit node per cursor (capped at
                // MAX_ACCESSKIT_CURSORS — RISK-004 / MC-004), nested under the text node so a swarm agent
                // can address each caret by `code_editor_cursor_{n}`. (The contract named `Role::TextCursor`,
                // which does not exist in accesskit 0.21; `Role::Caret` is the field-correct caret role —
                // see `emit_cursor_nodes` for the documented deviation.)
                self.emit_cursor_nodes(ui);

                // MT-005 AC-005: emit one `Role::TreeItem` AccessKit node per foldable region whose start
                // line is ACTUALLY PAINTED (capped at MAX_ACCESSKIT_FOLDS — RISK-001), with an
                // Expand/Collapse action reflecting the fold state, so a swarm agent can fold/unfold each
                // region by `code_editor_fold_{start_line}`. Nested under the text node like the cursors.
                self.emit_fold_nodes(ui, &painted_lines);
            },
        );
    }

    // ── MT-054 word-wrap rendering ────────────────────────────────────────────────────────────────

    /// MT-054 PERF CAP (adversarial-review hardening): ensure the cached [`WrapRowIndex`] is current for
    /// the live `(buffer_version, fold_version, wrap config, glyph_width, visible_lines)` key, rebuilding
    /// it ONLY on a key miss. The index is a prefix-sum of per-visible-line visual-row COUNTS — it is the
    /// single source of truth for the `show_rows` total-row count and for mapping a visual-row index back
    /// to its visible line, WITHOUT materializing the whole post-fold document's VisualRow list. Returns
    /// the total visual-row count.
    ///
    /// On a cache HIT (the common scroll / hover / idle repaint) this is O(1). On a MISS (edit / fold /
    /// wrap toggle / column / viewport-width / glyph-width change) it walks the visible lines once to
    /// count each line's fragments via [`count_visual_rows_for_line`] (O(document), but only when an input
    /// actually changed — never per frame). This is what stops the per-frame O(document) re-wrap the
    /// review caught: the per-FRAME paint path materializes only the painted window's lines.
    fn ensure_wrap_row_index(
        &self,
        visible_lines: usize,
        cfg: &WrapConfig,
        glyph_width: f32,
    ) -> usize {
        let key = WrapRowIndexKey {
            buffer_version: self.buffer_version.load(Ordering::Relaxed),
            fold_version: self.fold_version.load(Ordering::Relaxed),
            visible_lines,
            wrap_enabled: cfg.enabled,
            wrap_column: cfg.wrap_column,
            viewport_width_bits: cfg.viewport_width_px.to_bits(),
            glyph_width_bits: glyph_width.to_bits(),
        };
        let mut guard = self
            .wrap_row_index
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(idx) = guard.as_ref() {
            if idx.key == key {
                return idx.total_rows();
            }
        }
        // MISS: rebuild the prefix-sum of per-visible-line visual-row counts. `cumulative[i]` is the
        // visual-row count of visible lines 0..i, so `cumulative[visible_lines]` is the total.
        let mut cumulative: Vec<usize> = Vec::with_capacity(visible_lines + 1);
        cumulative.push(0);
        {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            let mut running = 0usize;
            for visible_idx in 0..visible_lines {
                let buffer_line = set.visible_line_to_buffer_line(visible_idx);
                // A folded-region START line is exactly one collapsed summary row (never wrapped).
                let folded = matches!(set.region_starting_at(buffer_line), Some(r) if r.folded);
                let n = if folded {
                    1
                } else {
                    count_visual_rows_for_line(&buffer, buffer_line, cfg, glyph_width)
                };
                running += n;
                cumulative.push(running);
            }
        }
        let total = *cumulative.last().unwrap_or(&0);
        *guard = Some(WrapRowIndex { key, cumulative });
        total
    }

    /// MT-054 PERF CAP: materialize the [`VisualRow`]s for ONLY the painted visual-row window
    /// `row_range` (in visual-row space), using the cached [`WrapRowIndex`] to translate the window into
    /// the slice of visible lines that intersect it. Per-frame cost is O(painted window), NOT O(document):
    /// only the logical lines that actually appear on screen are byte-materialized + wrapped this frame.
    ///
    /// Returns `(rows, window_start_visual, logical_lines_touched)` where `rows` are the visual rows whose
    /// indices fall in `row_range`, `window_start_visual` is the visual-row index of `rows[0]` (so paint y
    /// = `(idx - window_start_visual)`), and `logical_lines_touched` is the count fed to the perf
    /// diagnostic so a test can assert the paint stayed bounded.
    fn wrap_rows_for_window(
        &self,
        row_range: std::ops::Range<usize>,
        cfg: &WrapConfig,
        glyph_width: f32,
    ) -> (Vec<VisualRow>, usize, usize) {
        let guard = self
            .wrap_row_index
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(index) = guard.as_ref() else {
            return (Vec::new(), row_range.start, 0);
        };
        let total = index.total_rows();
        let want_start = row_range.start.min(total);
        let want_end = row_range.end.min(total);
        if want_start >= want_end {
            return (Vec::new(), want_start, 0);
        }
        // Which visible-line slot owns the first painted visual row, and the visual-row index that slot
        // begins at (so the first painted row may be a continuation fragment of a partly-scrolled line).
        let Some((first_slot, first_slot_start)) = index.visible_line_for_row(want_start) else {
            return (Vec::new(), want_start, 0);
        };

        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());

        // Walk visible-line slots from `first_slot`, materializing each line's fragments, until we have
        // covered `want_end`. We materialize the whole of `first_slot`'s line (cheap — one line) and trim
        // to the window afterwards, because the window may start mid-line.
        let mut all_rows: Vec<VisualRow> = Vec::new();
        let mut visual_cursor = first_slot_start; // visual-row index of the next row we push
        let mut slot = first_slot;
        let mut logical_lines_touched = 0usize;
        while visual_cursor < want_end && slot < index.cumulative.len() - 1 {
            let buffer_line = set.visible_line_to_buffer_line(slot);
            let folded = matches!(set.region_starting_at(buffer_line), Some(r) if r.folded);
            logical_lines_touched += 1;
            if folded {
                let start = buffer.line_to_byte(buffer_line).unwrap_or(0);
                let end = buffer
                    .line_to_byte(buffer_line + 1)
                    .unwrap_or_else(|| buffer.len_bytes());
                all_rows.push(VisualRow {
                    logical_line: buffer_line,
                    byte_start: start,
                    byte_end: end,
                    wrap_index: 0,
                });
                visual_cursor += 1;
            } else {
                let line_rows =
                    layout_visual_rows(&buffer, buffer_line..buffer_line + 1, cfg, glyph_width);
                visual_cursor += line_rows.len();
                all_rows.extend(line_rows);
            }
            slot += 1;
        }
        drop(set);
        drop(buffer);

        // `all_rows` covers visual indices `[first_slot_start, visual_cursor)`. Trim to `[want_start,
        // want_end)` so the returned rows align exactly with the painted window (the first fragment of a
        // partly-scrolled line is dropped when the window starts mid-line).
        let trim_front = want_start - first_slot_start;
        let trim_back_extra = visual_cursor.saturating_sub(want_end);
        let keep_end = all_rows.len().saturating_sub(trim_back_extra);
        let trimmed: Vec<VisualRow> = all_rows
            .into_iter()
            .skip(trim_front)
            .take(keep_end.saturating_sub(trim_front))
            .collect();
        (trimmed, want_start, logical_lines_touched)
    }

    /// MT-054: paint the ALREADY-WINDOWED visual rows under word wrap. `window_rows` are exactly the
    /// visual rows egui's `show_rows` asked for this frame (materialized lazily by
    /// [`wrap_rows_for_window`](Self::wrap_rows_for_window) — O(window), NOT O(document)); `window_start`
    /// is the GLOBAL visual-row index of `window_rows[0]` (for the scroll/geometry seam). Each visual row
    /// is one fragment of a logical line; the fragment text is painted on its own row, decorations overlay
    /// the painted window, and the AccessKit text node is emitted (the same nesting as `render_rows`).
    /// Indent guides are drawn ONLY for `wrap_index == 0` rows (RISK-007 / MC-007 — a continuation row has
    /// no real leading whitespace, so a guide there would be a ghost guide).
    #[allow(clippy::too_many_arguments)]
    fn render_wrapped_rows(
        &self,
        ui: &mut egui::Ui,
        window_rows: &[VisualRow],
        window_start: usize,
        syntax: &HsSyntaxTokens,
        total_lines: usize,
        text_id: egui::Id,
        text_author: &str,
        line_height: f32,
        glyph_width: f32,
    ) {
        ui.scope_builder(
            egui::UiBuilder::new()
                .id_salt(text_id)
                .sense(egui::Sense::focusable_noninteractive()),
            |ui| {
                let text_node_id = ui.unique_id();
                // WP-KERNEL-012 MT-080: record the LIVE text-node egui id so `consume_swarm_text_actions` reads
                // swarm SetValue/ReplaceSelectedText requests at the EXACT node the tree emitted (AC-080-6).
                *self
                    .live_text_node_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(text_node_id);
                ui.style_mut().spacing.item_spacing.y = 0.0;
                // MT-054 ROW-PITCH UNIT FIX (same as `render_rows`): pin the `ui.horizontal` frame floor to
                // exactly `line_height` so each painted visual row advances exactly one `line_height` — the
                // same unit the wrap-row index, `show_rows`, and the decoration y mapping stride by.
                ui.style_mut().spacing.interact_size.y = line_height;
                let origin = ui.cursor().min;

                // The buffer-line span the painted visual rows cover, for the highlight-span byte window.
                let (first_buffer_line, last_buffer_line) = if !window_rows.is_empty() {
                    (
                        window_rows[0].logical_line,
                        window_rows[window_rows.len() - 1].logical_line,
                    )
                } else {
                    (0, 0)
                };
                let buffer_end = (last_buffer_line + 1).min(total_lines);
                let (win_start, win_end) = self.with_buffer(|b| {
                    let ws = b.line_to_byte(first_buffer_line).unwrap_or(0);
                    let we = b.line_to_byte(buffer_end).unwrap_or_else(|| b.len_bytes());
                    (ws, we)
                });
                let visible_spans = self.spans_in_byte_window(win_start, win_end);

                // Paint each visual-row fragment as its own row (the fragment's byte slice, syntax-colored).
                for row in window_rows {
                    let folded_label = {
                        let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
                        match set.region_starting_at(row.logical_line) {
                            Some(r) if r.folded && row.wrap_index == 0 => Some(r.label.clone()),
                            _ => None,
                        }
                    };
                    match folded_label {
                        Some(label) => self.render_fold_label_line(ui, &label, syntax),
                        None => self.render_visual_row_fragment(ui, row, &visible_spans, syntax),
                    }
                }
                *self
                    .last_gutter_paint_rows
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = window_rows
                    .iter()
                    .map(|row| GutterPaintRow {
                        line: row.logical_line,
                        is_first_fragment: row.is_first_fragment(),
                    })
                    .collect();

                // The painted window's RowGeometry: `first_line` is the GLOBAL visual-row index of the first
                // painted row (NOT a buffer line) because under wrap the rows are in visual space. The
                // decoration painters map a byte offset to a row by its position WITHIN `window_rows`, whose
                // index 0 is at `geometry.top`, so the y mapping stays correct for the windowed slice.
                let geometry = RowGeometry {
                    left: origin.x,
                    top: origin.y,
                    first_line: window_start,
                    line_height,
                };
                *self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()) = Some(geometry);

                // MT-054 decorations under wrap: indent guides only on first-fragment rows, bracket colors +
                // match highlight mapped through the painted visual-row window (RISK-007 / MC-007). The
                // `painted_lines` slice is empty because the wrap arm maps bytes through `window_rows`
                // directly (each visual row carries its own byte range — already fold-filtered + bounded).
                self.paint_chrome_decorations(ui, &geometry, glyph_width, &[], Some(window_rows));

                // MT-054 Task-B: paint the SAME overlays the non-wrap `render_rows` paints, made wrap-aware
                // (byte -> visual row via `window_rows`). Before this the wrap path painted none of them, so
                // Alt+Z wrap ON showed NO caret, selection, find-match highlight, or render-whitespace glyphs.
                // `painted_lines` is empty (`&[]`) — the overlays map through `Some(window_rows)`; the paint
                // stays bounded to the on-screen visual-row window.
                if self.render_whitespace() {
                    self.paint_whitespace_glyphs(
                        ui,
                        &geometry,
                        glyph_width,
                        &[],
                        syntax,
                        Some(window_rows),
                    );
                }
                self.paint_match_highlights(ui, &geometry, glyph_width, &[], Some(window_rows));
                self.paint_cursor_overlay(
                    ui,
                    &geometry,
                    glyph_width,
                    &[],
                    syntax,
                    Some(window_rows),
                );

                let author = text_author.to_owned();
                let painted_fold_lines: Vec<usize> = window_rows
                    .iter()
                    .filter(|row| row.is_first_fragment())
                    .map(|row| row.logical_line)
                    .collect();
                self.emit_fold_nodes(ui, &painted_fold_lines);

                ui.ctx().accesskit_node_builder(text_node_id, move |node| {
                    node.set_role(accesskit::Role::TextInput);
                    node.set_author_id(author.clone());
                    node.set_label("Code editor text".to_owned());
                    node.set_value(format!("{total_lines} lines (word wrap on)"));
                    // Word wrap changes layout only; it must not remove the same model-facing authoring
                    // contract exposed by the unwrapped TextInput.
                    node.add_action(accesskit::Action::SetValue);
                    node.add_action(accesskit::Action::ReplaceSelectedText);
                    node.add_action(accesskit::Action::Focus);
                    node.add_action(accesskit::Action::Click);
                });

                // MT-054 Task-B: emit one `Role::Caret` AccessKit node per cursor under wrap too (position-
                // independent — a swarm agent can still address each caret by `code_editor_cursor_{n}`), the
                // same node `render_rows` emits in the non-wrap path.
                self.emit_cursor_nodes(ui);
            },
        );
    }

    /// MT-054: paint ONE wrapped visual-row fragment (`row.byte_start..row.byte_end`) as a single row,
    /// syntax-colored from `visible_spans`. A continuation fragment (`wrap_index > 0`) is NOT re-indented
    /// (Monaco's default wrap indent is 0); the trailing newline on the final fragment is stripped so the
    /// row holds one visual line. Mirrors `render_line`'s run-splitting but over the fragment's byte
    /// window instead of a whole logical line.
    fn render_visual_row_fragment(
        &self,
        ui: &mut egui::Ui,
        row: &VisualRow,
        visible_spans: &HighlightSpanWindow,
        syntax: &HsSyntaxTokens,
    ) {
        let frag_start = row.byte_start;
        let frag_text_owned = self.with_buffer(|b| b.byte_slice_to_string(row.byte_range()));
        let frag_text = frag_text_owned
            .strip_suffix('\n')
            .unwrap_or(&frag_text_owned);
        let frag_end = frag_start + frag_text.len();

        let mono = self.mono_font();
        // MT-078: same RTL/limitation treatment as the non-wrap `render_line`, applied per VISIBLE wrap
        // fragment (the bidi cost is bounded to the rows on screen). A pure-LTR fragment returns `false`
        // and falls through to the EXACT existing per-run colored path (AC6 identity).
        if self.render_rtl_or_limited_code_row(ui, frag_text, &mono, syntax) {
            return;
        }

        let mut runs: Vec<(std::ops::Range<usize>, HighlightScope)> = Vec::new();
        for span in visible_spans.overlapping(frag_start, frag_end) {
            let s = span.byte_range.start.max(frag_start);
            let e = span.byte_range.end.min(frag_end);
            if s < e {
                runs.push((s..e, span.scope));
            }
        }
        runs.sort_by_key(|(r, _)| r.start);

        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let default_color = syntax.punctuation;
            let frag_slice = |start: usize, end: usize| -> String {
                let rel_start = start.saturating_sub(frag_start);
                let rel_end = end.saturating_sub(frag_start);
                if rel_start >= rel_end || rel_end > frag_text.len() {
                    return String::new();
                }
                let bytes = frag_text.as_bytes();
                let mut a = rel_start;
                while a < frag_text.len() && !frag_text.is_char_boundary(a) {
                    a += 1;
                }
                let mut b = rel_end.min(frag_text.len());
                while b < frag_text.len() && !frag_text.is_char_boundary(b) {
                    b += 1;
                }
                if a >= b {
                    return String::new();
                }
                std::str::from_utf8(&bytes[a..b]).unwrap_or("").to_owned()
            };

            let mut cursor = frag_start;
            for (range, scope) in &runs {
                if range.start > cursor {
                    let gap = frag_slice(cursor, range.start);
                    if !gap.is_empty() {
                        Self::code_static_label(
                            ui,
                            egui::RichText::new(gap)
                                .font(mono.clone())
                                .color(default_color),
                        );
                    }
                }
                let run_text = frag_slice(range.start, range.end);
                if !run_text.is_empty() {
                    let color = self.resolve_highlight_color(*scope, syntax);
                    Self::code_static_label(
                        ui,
                        egui::RichText::new(run_text)
                            .font(mono.clone())
                            .color(color),
                    );
                }
                cursor = cursor.max(range.end);
            }
            if cursor < frag_end {
                let tail = frag_slice(cursor, frag_end);
                if !tail.is_empty() {
                    Self::code_static_label(
                        ui,
                        egui::RichText::new(tail)
                            .font(mono.clone())
                            .color(default_color),
                    );
                }
            }
            if runs.is_empty() && frag_text.is_empty() {
                Self::code_static_label(
                    ui,
                    egui::RichText::new(" ")
                        .font(mono.clone())
                        .color(default_color),
                );
            }
        });
        // Same duplicate-label fix as `render_line` (this is its word-wrap fragment variant): the row is a
        // structural wrapper around the per-run text labels, not a second text label. A GenericContainer
        // role avoids accesskit deriving the container name from the fragment text (which duplicated the
        // inner label), so each wrapped fragment exposes exactly one text label.
        ui.ctx().accesskit_node_builder(row.response.id, |node| {
            node.set_role(accesskit::Role::GenericContainer);
        });
    }

    /// MT-054: paint the editor-chrome decorations over the painted row window — vertical indent guides,
    /// bracket-pair colorization, and the matching-bracket highlight box. Theme-sourced (the indent-guide
    /// tokens + bracket-pair palette come from `theme/palette.rs`; this fn holds NO color literal —
    /// CONTROL-4). Render-only (no buffer mutation — AC-007).
    ///
    /// `first_buffer_line..end_line` is the BUFFER-line window painted. `wrap_rows`:
    ///   - `None` (non-wrap path): each painted row is one logical line at buffer-line
    ///     `geometry.first_line + offset`; guides/brackets map a buffer (line,col) to y via the buffer
    ///     line index.
    ///   - `Some(rows)` (wrap path): the painted rows are the given visual rows (in visual order); a
    ///     decoration's row y is the visual-row index. Indent guides are drawn ONLY for `wrap_index == 0`
    ///     rows (RISK-007 / MC-007).
    fn paint_chrome_decorations(
        &self,
        ui: &egui::Ui,
        geometry: &RowGeometry,
        glyph_width: f32,
        painted_lines: &[usize],
        wrap_rows: Option<&[VisualRow]>,
    ) {
        // Resolve the theme tokens (dark/light) for the guides + bracket palette.
        let palette = if ui.visuals().dark_mode {
            crate::theme::HsTheme::Dark.palette()
        } else {
            crate::theme::HsTheme::Light.palette()
        };
        let guide_color = palette.indent_guide;
        let active_guide_color = palette.indent_guide_active;
        let bracket_palette = palette.bracket_pair_palette.clone();
        let (tab_width, _) = self.indent_settings();
        let tab_width = tab_width.max(1);

        let painter = ui.painter();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());

        // The active indent level = the indent level of the cursor's current logical line (the block the
        // cursor is in). The guide AT that level is drawn in the active color (VS Code semantics).
        let cursor_line = {
            let set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            byte_to_line_col(set.primary().head, &buffer).0
        };
        let active_level = indent_level_of(&buffer, cursor_line, tab_width);

        // 1) INDENT GUIDES (drawn first — faint vertical lines in the whitespace columns).
        //    Non-wrap: one row per buffer line in [first_buffer_line, end_line); the row y for a buffer
        //    line is (line - first_buffer_line). Wrap: one row per visual row; only wrap_index==0 rows
        //    carry guides, and the row y is the visual-row offset.
        let paint_guides_for = |row_offset: usize, buffer_line: usize| {
            let level = indent_level_of(&buffer, buffer_line, tab_width);
            if level == 0 {
                return;
            }
            let y0 = geometry.top + row_offset as f32 * geometry.line_height;
            let y1 = y0 + geometry.line_height;
            for lvl in 1..=level {
                let x = indent_guide_x(geometry.left, lvl, tab_width, glyph_width);
                // The guide at the cursor's indent level is the ACTIVE guide (VS Code highlights the
                // guide of the block enclosing the cursor across that block). `active_level` is the
                // cursor line's indent level; any painted line that is indented at least that deep draws
                // its level-`active_level` guide in the active color so the enclosing block reads.
                let color = if active_level > 0 && lvl == active_level {
                    active_guide_color
                } else {
                    guide_color
                };
                painter.vline(x, y0..=y1, egui::Stroke::new(1.0, color));
            }
        };
        // MT-035 wave-7: the indent-guide pass is gated by the `editor_prefs.indent_guides` toggle the shell
        // threads in via `set_indent_guides_enabled`. When OFF, no guides are painted (and
        // `indent_guide_count_for_line` reports 0) — the toggle drives a real, visible chrome change.
        if self.indent_guides_enabled() {
            match wrap_rows {
                None => {
                    // FOLD-AWARE (Wave-B fix): the painted rows are exactly `painted_lines` — one buffer
                    // line per row, in row order, with fold-hidden lines absent. Enumerating THIS list (not
                    // a contiguous `first..end` line range, which the old code wrongly assumed mapped 1:1 to
                    // row offsets) puts every guide on its real painted row and never draws a hidden line's
                    // guide.
                    for (row_offset, &line) in painted_lines.iter().enumerate() {
                        paint_guides_for(row_offset, line);
                    }
                }
                Some(rows) => {
                    for (row_offset, row) in rows.iter().enumerate() {
                        if row.is_first_fragment() {
                            paint_guides_for(row_offset, row.logical_line);
                        }
                    }
                }
            }
        }

        // 2) BRACKET-PAIR COLORIZATION: re-draw each bracket glyph in its depth color over the painted
        //    text (z-order: above guides + text). FOLD-AWARE (Wave-B fix): the scan runs over the
        //    VISIBLE rows' byte segments ONLY — a folded region's hidden bytes are never scanned, so no
        //    hidden bracket is ever colored (and the per-frame cost is O(visible bytes) even when a
        //    huge folded region sits inside the window). Depth carries ACROSS the segments (as if the
        //    hidden text were absent), consistent with the documented window-relative depth.
        let visible_segments: Vec<std::ops::Range<usize>> = match wrap_rows {
            None => {
                let mut segs: Vec<std::ops::Range<usize>> = Vec::new();
                for &line in painted_lines {
                    let s = buffer.line_to_byte(line).unwrap_or(0);
                    let e = buffer
                        .line_to_byte(line + 1)
                        .unwrap_or_else(|| buffer.len_bytes());
                    match segs.last_mut() {
                        Some(last) if last.end == s => last.end = e,
                        _ => segs.push(s..e),
                    }
                }
                segs
            }
            Some(rows) => {
                let mut segs: Vec<std::ops::Range<usize>> = Vec::new();
                for row in rows {
                    let (s, e) = (row.byte_start, row.byte_end);
                    match segs.last_mut() {
                        Some(last) if last.end == s => last.end = e,
                        _ => segs.push(s..e),
                    }
                }
                segs
            }
        };
        if !bracket_palette.is_empty() {
            let colors =
                bracket_pair_colors_in_segments(&buffer, &visible_segments, &bracket_palette);
            let mono = self.mono_font();
            for (range, color) in colors {
                if let Some((x, y)) = self.decoration_xy(
                    &buffer,
                    range.start,
                    geometry,
                    glyph_width,
                    painted_lines,
                    wrap_rows,
                ) {
                    let ch = buffer.byte_slice_to_string(range.clone());
                    if !ch.is_empty() {
                        painter.text(
                            egui::pos2(x, y),
                            egui::Align2::LEFT_TOP,
                            ch,
                            mono.clone(),
                            color,
                        );
                    }
                }
            }
        }

        // 3) MATCHING-BRACKET HIGHLIGHT: a rounded box behind the two matched brackets when the cursor is
        //    adjacent to a bracket (VS Code adjacency). Painted last so it sits on top. MT-035 wave-7: gated
        //    by the `editor_prefs.bracket_matching` toggle via the shared `matching_bracket_pair` helper —
        //    when OFF it returns None and no box is painted (the same gate `matching_bracket_at` exposes to
        //    tests, so render + test never drift).
        let cursor_byte = {
            let set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            set.primary().head
        };
        if let Some((open_byte, close_byte)) = self.matching_bracket_pair(&buffer, cursor_byte) {
            let stroke = egui::Stroke::new(1.0, active_guide_color);
            for b in [open_byte, close_byte] {
                if let Some((x, y)) =
                    self.decoration_xy(&buffer, b, geometry, glyph_width, painted_lines, wrap_rows)
                {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(x, y),
                        egui::vec2(glyph_width, geometry.line_height),
                    );
                    painter.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
                }
            }
        }
    }

    /// MT-054/MT-005 FOLD-AWARE ROW MAP: the PAINTED row offset of `buffer_line` in the current window,
    /// or `None` when the line is fold-hidden or off-window. `painted_lines` is the strictly-ascending
    /// buffer line per painted row `render_rows` recorded this frame — the ONE line->row map every
    /// overlay/decoration painter shares (never `line - first_line`, which drifts across folds).
    fn painted_row_offset(painted_lines: &[usize], buffer_line: usize) -> Option<usize> {
        painted_lines.binary_search(&buffer_line).ok()
    }

    /// MT-054: map an absolute buffer byte offset to the (x, y) top-left of its glyph in the painted row
    /// window, or `None` if the offset is not on a painted row. Non-wrap: the row y is the buffer line's
    /// PAINTED row offset in `painted_lines` (fold-aware — a fold-hidden line is not in the list, so its
    /// decoration is skipped, and the mapping is bounded above AND below by the painted window — the
    /// Wave-B fix); the column is the char offset within the line. Wrap: find the visual row whose byte
    /// fragment contains the offset and use its visual-row index for y + the offset within the fragment
    /// for the column. Reuses the SAME `glyph_width` + `line_height` units the rows were painted with
    /// (RISK-002 / MC-002 — no independent metric recompute).
    fn decoration_xy(
        &self,
        buffer: &TextBuffer,
        byte_offset: usize,
        geometry: &RowGeometry,
        glyph_width: f32,
        painted_lines: &[usize],
        wrap_rows: Option<&[VisualRow]>,
    ) -> Option<(f32, f32)> {
        match wrap_rows {
            None => {
                let (line, col) = byte_to_line_col(byte_offset, buffer);
                let row_offset = Self::painted_row_offset(painted_lines, line)?;
                let x = geometry.left + col as f32 * glyph_width;
                let y = geometry.top + row_offset as f32 * geometry.line_height;
                Some((x, y))
            }
            Some(rows) => {
                // Find the visual row whose fragment covers byte_offset.
                let idx = rows
                    .iter()
                    .position(|r| byte_offset >= r.byte_start && byte_offset < r.byte_end)?;
                let row = &rows[idx];
                // Column within the fragment = chars between the fragment start and the offset.
                let frag = buffer.byte_slice_to_string(row.byte_start..byte_offset);
                let col = frag.chars().count();
                let x = geometry.left + col as f32 * glyph_width;
                let y = geometry.top + idx as f32 * geometry.line_height;
                Some((x, y))
            }
        }
    }

    /// MT-054 Task-B: the (x, y) top-left of `byte_offset`'s cell under WORD WRAP — the painted VISUAL-row
    /// index (not the logical line) drives y and the column is the char count within the fragment. This is
    /// the overlay analogue of `decoration_xy`'s `Some(rows)` arm, so the caret / preedit overlays land on
    /// the same wrapped row the text fragment paints on. Returns `None` when the byte is off the painted
    /// window. A byte exactly at a fragment end (a caret at the line/content edge) maps to that fragment's
    /// last column; a trailing `\n` in the fragment is not counted as a column.
    fn wrap_overlay_pos(
        rows: &[VisualRow],
        buffer: &TextBuffer,
        byte_offset: usize,
        geometry: &RowGeometry,
        glyph_width: f32,
    ) -> Option<(f32, f32)> {
        let idx = rows
            .iter()
            .position(|r| byte_offset >= r.byte_start && byte_offset < r.byte_end)
            .or_else(|| rows.iter().rposition(|r| byte_offset == r.byte_end))?;
        let row = &rows[idx];
        let seg = buffer.byte_slice_to_string(row.byte_start..byte_offset.min(row.byte_end));
        let mut col = seg.chars().count();
        if seg.ends_with('\n') {
            col = col.saturating_sub(1);
        }
        let x = geometry.left + col as f32 * glyph_width;
        let y = geometry.top + idx as f32 * geometry.line_height;
        Some((x, y))
    }

    /// MT-054 Task-B: the overlay rects covering byte range `[start, end)` under WORD WRAP — one rect per
    /// painted VISUAL row the range intersects (the wrapped analogue of the per-line selection / find-match
    /// rects in the non-wrap overlays). A trailing `\n` inside a fragment is not counted as a column, so a
    /// whole-line selection stops at the content edge. Bounded to the painted `rows` window.
    fn wrap_overlay_rects(
        rows: &[VisualRow],
        buffer: &TextBuffer,
        range: std::ops::Range<usize>,
        geometry: &RowGeometry,
        glyph_width: f32,
    ) -> Vec<egui::Rect> {
        let mut out: Vec<egui::Rect> = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            let s = range.start.max(row.byte_start);
            let e = range.end.min(row.byte_end);
            if s >= e {
                continue;
            }
            let start_col = buffer
                .byte_slice_to_string(row.byte_start..s)
                .chars()
                .count();
            let end_seg = buffer.byte_slice_to_string(row.byte_start..e);
            let mut end_col = end_seg.chars().count();
            if end_seg.ends_with('\n') {
                end_col = end_col.saturating_sub(1);
            }
            if end_col <= start_col {
                continue;
            }
            let y = geometry.top + idx as f32 * geometry.line_height;
            let x0 = geometry.left + start_col as f32 * glyph_width;
            let x1 = geometry.left + end_col as f32 * glyph_width;
            out.push(egui::Rect::from_min_max(
                egui::pos2(x0, y),
                egui::pos2(x1, y + geometry.line_height),
            ));
        }
        out
    }

    /// Render a folded region's collapsed SUMMARY line (the start-line text + ` …`) in place of the
    /// region's real lines (MT step 4). One row, monospace, in the editor foreground color — the same
    /// row height as a real line so the virtualized layout stays on one unit. A subtle background tint
    /// (the theme comment color at low alpha) marks it as a fold summary without a new theme token.
    fn render_fold_label_line(&self, ui: &mut egui::Ui, label: &str, syntax: &HsSyntaxTokens) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mono = self.mono_font();
            // The label text in the normal foreground; the trailing ellipsis already conveys "folded".
            let resp = ui.label(
                egui::RichText::new(label)
                    .font(mono)
                    .color(syntax.punctuation),
            );
            // A faint highlight rect behind the summary so a folded line reads differently from a normal
            // line (UI affordance, like the find-match tint — not a syntax token).
            let tint = egui::Color32::from_rgba_unmultiplied(
                syntax.comment.r(),
                syntax.comment.g(),
                syntax.comment.b(),
                28,
            );
            ui.painter().rect_filled(resp.rect, 0.0, tint);
        });
    }

    /// Emit the per-fold-region `Role::TreeItem` AccessKit nodes for regions whose start line is in the
    /// actual painted row list (AC-005 / HBR-SWARM). This deliberately keys off painted rows rather than
    /// a broad buffer span so a nested region hidden inside a folded outer region never emits a live node.
    /// Each node:
    /// - author_id `code_editor_fold_{start_line}` (the contract-named id; AC-005 asserts THIS id),
    /// - role `Role::TreeItem` (exists in accesskit 0.21 — no fallback needed; verified at build),
    /// - action `Action::Expand` when the region is FOLDED (the agent action that unfolds it) or
    ///   `Action::Collapse` when UNFOLDED (the agent action that folds it) — MT impl note "accessible
    ///   fold state",
    /// - value carries the fold state + line span so an agent can read it without dispatching.
    ///
    /// Capped at [`MAX_ACCESSKIT_FOLDS`] nodes (RISK-001) so a file with thousands of folds cannot blow
    /// the per-frame node budget. Fold ids are keyed by buffer start line so NodeIds stay stable across
    /// frames and cannot be reassigned to a different visible slot after scrolling; instances hash the
    /// suffixed author_id (RISK-004), the same scheme the cursor nodes use.
    fn emit_fold_nodes(&self, ui: &egui::Ui, painted_lines: &[usize]) {
        let regions: Vec<(usize, usize, bool)> = {
            let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            painted_lines
                .iter()
                .filter_map(|&line| set.region_starting_at(line))
                .take(MAX_ACCESSKIT_FOLDS)
                .map(|r| (r.start_line, r.end_line, r.folded))
                .collect()
        };
        let mut requested_states: Vec<(usize, bool)> = Vec::new();
        for (slot, (start_line, end_line, folded)) in regions.into_iter().enumerate() {
            let author = if self.instance.is_empty() {
                format!("{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}")
            } else {
                format!(
                    "{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}#{}",
                    self.instance
                )
            };
            let value = if folded {
                format!("folded lines {start_line}-{end_line}")
            } else {
                format!("unfolded lines {start_line}-{end_line}")
            };
            let node_id = self.fold_node_id(slot, start_line);
            ui.ctx().accesskit_node_builder(node_id, move |node| {
                node.set_role(accesskit::Role::TreeItem);
                node.set_author_id(author.clone());
                node.set_label("Code editor fold".to_owned());
                node.set_value(value.clone());
                // The action an agent dispatches to CHANGE the state: Expand un-folds a folded region;
                // Collapse folds an unfolded one (AC-005: a FOLDED region's node supports Expand).
                if folded {
                    node.add_action(accesskit::Action::Expand);
                } else {
                    node.add_action(accesskit::Action::Collapse);
                }
            });
            ui.input(|input| {
                if folded {
                    if input
                        .accesskit_action_requests(node_id, accesskit::Action::Expand)
                        .next()
                        .is_some()
                    {
                        requested_states.push((start_line, false));
                    }
                } else if input
                    .accesskit_action_requests(node_id, accesskit::Action::Collapse)
                    .next()
                    .is_some()
                {
                    requested_states.push((start_line, true));
                }
            });
        }
        for (start_line, folded) in requested_states {
            if self.set_fold_state(start_line, folded) {
                ui.ctx().request_repaint();
            }
        }
    }

    /// The stable `egui::Id` for a fold node. It is keyed by `start_line`, not by the currently painted
    /// slot, so a stale AccessKit action cannot hit a different fold row after scrolling or virtualization
    /// changes the visible slot order.
    fn fold_node_id(&self, _slot: usize, start_line: usize) -> egui::Id {
        if self.instance.is_empty() {
            egui::Id::new(format!("{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}"))
        } else {
            egui::Id::new(format!(
                "{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}#{}",
                self.instance
            ))
        }
    }

    // ── MT-007 gutter render + AccessKit ──────────────────────────────────────────────────────────

    /// Paint the gutter strip content into `gutter_rect` after the editor rows painted this frame, then
    /// apply any fold/breakpoint click and emit the per-line breakpoint/diagnostic AccessKit nodes.
    /// Reads the captured [`RowGeometry`] so the gutter aligns row-for-row with the code body (the SAME
    /// `origin`/`line_height` the rows were painted at), and reads the painted VISIBLE window mapped to
    /// BUFFER lines through the fold set so a folded region shifts the gutter rows in lockstep with the
    /// editor (MT-005 fold-aware mapping). A no-op (clears the captured rows) when no frame geometry is
    /// available yet.
    fn render_gutter(
        &self,
        ui: &mut egui::Ui,
        gutter_rect: egui::Rect,
        glyph_width: f32,
        config: &GutterConfig,
        editor_container_clip: egui::Rect,
    ) {
        // The painted-row geometry captured by `render_rows` this frame (origin = top-left of the first
        // painted code row; line_height = sans-spacing row stride). Without it (no frame yet) there is
        // nothing to align to.
        let Some(row_geom) = *self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()) else {
            self.last_gutter_rows
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            self.last_gutter_paint_rows
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clear();
            *self
                .last_gutter_geometry
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            return;
        };

        // The exact painted body rows for this frame. Rebuilding these from `last_visible_range` is only
        // correct when wrap is off; under wrap that range is visual-row space and a long first logical
        // line may occupy rows 0..N. The body paint path already knows the right row model, so consume it.
        let paint_rows = self
            .last_gutter_paint_rows
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let first_fragment_lines: Vec<usize> = paint_rows
            .iter()
            .filter(|row| row.is_first_fragment)
            .map(|row| row.line)
            .collect();

        // The gutter geometry: origin at the gutter strip's left edge + the code rows' top, with the
        // editor's measured line height + glyph width (so the line numbers use the SAME metrics).
        let geometry = GutterGeometry {
            origin: egui::pos2(gutter_rect.left(), row_geom.top),
            line_height: row_geom.line_height,
            char_width: glyph_width,
            font_size: self.font_size(),
        };

        // Snapshot the markers + breakpoints (clones so no lock is held across egui calls).
        let markers = self
            .diagnostic_markers
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let breakpoints = self
            .breakpoint_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        // A closure the gutter calls to learn whether a buffer line starts a fold region and, if so,
        // whether it is OPEN (not folded). `Some(true)` = region start, expanded; `Some(false)` = region
        // start, collapsed; `None` = not a region start (no triangle).
        let fold_open_for = |line: usize| -> Option<bool> {
            let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            set.region_starting_at(line).map(|r| !r.folded)
        };

        let buffer = self
            .buffer
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        // `SidePanel::show_inside` has already reserved the gutter from this parent UI's available
        // rect. `render_gutter` runs after the scroll body so it can consume the exact painted-row
        // geometry, but `Ui::interact` clips every hit rect to the CURRENT UI clip. Expand the clip
        // back over the reserved strip only while registering gutter widgets; otherwise their painted
        // rects remain visible while their interact rects become negative/empty and real pointer clicks
        // never reach breakpoint/fold controls. Restore the original clip immediately afterward.
        let original_clip = ui.clip_rect();
        ui.set_clip_rect(editor_container_clip.intersect(gutter_rect));
        let response: GutterResponse = Gutter::render(
            ui,
            gutter_rect,
            &paint_rows,
            &buffer,
            &markers,
            &breakpoints,
            config,
            geometry,
            &fold_open_for,
        );
        if config.show_breakpoints {
            for &line in &first_fragment_lines {
                let target_id = ui.id().with(("gutter_row", line)).with("bp");
                let author = if self.instance.is_empty() {
                    format!("{CODE_EDITOR_BREAKPOINT_TARGET_AUTHOR_PREFIX}{line}")
                } else {
                    format!(
                        "{CODE_EDITOR_BREAKPOINT_TARGET_AUTHOR_PREFIX}{line}#{}",
                        self.instance
                    )
                };
                ui.ctx().accesskit_node_builder(target_id, move |node| {
                    node.set_role(accesskit::Role::Button);
                    node.set_author_id(author.clone());
                    node.set_label(format!("Toggle breakpoint on line {}", line + 1));
                });
            }
        }
        if config.show_fold_triangles {
            for &line in &first_fragment_lines {
                if fold_open_for(line).is_none() {
                    continue;
                }
                let target_id = ui.id().with(("gutter_row", line)).with("fold");
                let author = if self.instance.is_empty() {
                    format!("{CODE_EDITOR_FOLD_TARGET_AUTHOR_PREFIX}{line}")
                } else {
                    format!(
                        "{CODE_EDITOR_FOLD_TARGET_AUTHOR_PREFIX}{line}#{}",
                        self.instance
                    )
                };
                ui.ctx().accesskit_node_builder(target_id, move |node| {
                    node.set_role(accesskit::Role::Button);
                    node.set_author_id(author.clone());
                    node.set_label(format!("Toggle fold on line {}", line + 1));
                });
            }
        }

        // Persist the painted rows + geometry for the deterministic click tests.
        *self
            .last_gutter_rows
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = paint_rows.iter().map(|row| row.line).collect();
        *self
            .last_gutter_geometry
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(geometry);

        // Apply the click outcomes AFTER the render (the same post-render-apply discipline the cursor
        // overlay + fold keymap use). A fold click toggles the fold; a breakpoint click toggles the
        // breakpoint AND publishes a BreakpointEvent (RISK-003 non-blocking publish).
        if let Some(line) = response.fold_toggled {
            self.toggle_fold(line);
        }
        if let Some(line) = response.breakpoint_toggled {
            self.toggle_breakpoint(line);
        }

        // Emit the per-line breakpoint (CheckBox) + diagnostic (Label) AccessKit nodes for the painted
        // rows so a swarm agent can address each by `code_editor_breakpoint_{line}` /
        // `code_editor_diagnostic_{line}` and toggle/read it by id (AC-005 / HBR-SWARM). Capped per
        // frame (RISK-004) and restricted to the painted window so a huge file cannot blow the node
        // budget.
        self.emit_breakpoint_nodes(ui, &first_fragment_lines, &breakpoints, config);
        self.emit_diagnostic_nodes(ui, &first_fragment_lines, &markers, config);
        self.draw_diagnostic_note_reference_chips(ui, &paint_rows, &geometry, config);

        // MT-049: draw the quick-fix lightbulb on any PAINTED line that currently has available code
        // actions (AC-003 — only on the diagnostic line with actions, never on a line without). The glyph is
        // a clickable Role::Button that opens the quick-fix menu (the gutter-click path). Theme-aware
        // (CONTROL-4 — `lightbulb_color`/`warn_fg_color`, no Color32 literal). Only drawn when the menu is
        // closed so the bulb does not overdraw the open menu (the bulb stays "lit" via the controller state).
        self.draw_quickfix_lightbulbs(ui, &paint_rows, &geometry);
        ui.set_clip_rect(original_clip);
    }

    /// MT-046 IC-09: render one real related-note chip on each visible diagnostic line that carries a
    /// note destination. The click goes through the canonical shared-bus `open-document` command, so
    /// the mounted shell selects/focuses the real rich editor rather than a test-only navigation seam.
    fn draw_diagnostic_note_reference_chips(
        &self,
        ui: &mut egui::Ui,
        paint_rows: &[GutterPaintRow],
        geometry: &GutterGeometry,
        config: &GutterConfig,
    ) {
        if !config.show_diagnostics {
            return;
        }
        let references = self
            .diagnostic_note_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        for (painted_idx, row) in paint_rows.iter().enumerate() {
            if !row.is_first_fragment {
                continue;
            }
            let Some(document_id) = references.get(&row.line).cloned() else {
                continue;
            };
            let author = self.diagnostic_note_reference_author_id(row.line);
            let row_top = geometry.origin.y + painted_idx as f32 * geometry.line_height;
            let size = geometry.line_height.min(16.0);
            let rect = egui::Rect::from_min_size(
                egui::pos2(geometry.origin.x + 1.0, row_top),
                egui::vec2(size, size),
            );
            let response = ui
                .push_id(&author, |ui| {
                    ui.put(rect, egui::Button::new("↗").frame(false))
                })
                .inner
                .on_hover_text(format!("Open related note {document_id}"));
            let node_author = author.clone();
            let node_document = document_id.clone();
            ui.ctx().accesskit_node_builder(response.id, move |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(node_author.clone());
                node.set_label("Open diagnostic related note".to_owned());
                node.set_value(node_document.clone());
                node.add_action(accesskit::Action::Click);
            });
            if response.clicked() {
                let bus = crate::interop::InteractionBus::get_or_init(ui.ctx());
                let dispatched = crate::interop::InteractionBus::with_try_lock(&bus, |bus| {
                    bus.register_open_document_command();
                    bus.open_document(ui.ctx(), document_id.clone())
                });
                if dispatched != Some(true) {
                    ui.ctx().request_repaint();
                }
            }
        }
    }

    /// MT-049: draw the quick-fix lightbulb on each painted line that carries available code actions
    /// (AC-003). A click on the bulb opens the quick-fix menu (the gutter-click trigger). The bulb sits in
    /// the gutter's left margin, vertically centered on its row. Only the painted lines are considered so a
    /// huge file cannot draw an off-screen bulb. A clicked bulb opens the menu for that line.
    fn draw_quickfix_lightbulbs(
        &self,
        ui: &mut egui::Ui,
        paint_rows: &[GutterPaintRow],
        geometry: &GutterGeometry,
    ) {
        let mut open_for: Option<usize> = None;
        let mut drawn: Vec<(usize, egui::Pos2)> = Vec::new();
        for (painted_idx, row) in paint_rows.iter().enumerate() {
            if !row.is_first_fragment {
                continue;
            }
            let line = row.line;
            if !self.has_quickfix_on_line(line) {
                continue;
            }
            // Center the bulb on the row, INSIDE the gutter strip (MT-049 Wave-B fix: the old anchor
            // `origin.x + char_width * 0.6` put the glyph CENTER ~4.7px from the panel's left edge, so
            // the CENTER_CENTER-drawn ~13px glyph clipped half off the panel). Anchoring the center a
            // full half-glyph-plus-margin in keeps the whole bulb visible in the gutter's left column.
            let y = geometry.origin.y
                + painted_idx as f32 * geometry.line_height
                + geometry.line_height * 0.5;
            let x = geometry.origin.x + code_actions::LIGHTBULB_GLYPH_SIZE * 0.5 + 2.0;
            let pos = egui::pos2(x, y);
            let resp = code_actions::draw_lightbulb(ui, line, pos, &self.instance);
            drawn.push((line, pos));
            if resp.clicked() {
                open_for = Some(line);
            }
        }
        *self
            .last_quickfix_lightbulbs
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = drawn;
        // A clicked lightbulb opens the quick-fix menu for that line (AC-003 — gutter-click path). If a
        // request for the line is not yet resolved the menu opens empty and re-fires on the next pump.
        if let Some(line) = open_for {
            let mut controller = self
                .code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if controller.active_line() == Some(line) {
                controller.open_menu();
            }
        }
    }

    /// Emit one `Role::CheckBox` AccessKit node per PAINTED breakpoint line (toggled = the line carries
    /// a breakpoint), so a swarm agent can find + toggle each breakpoint by `code_editor_breakpoint_{line}`.
    /// `Role::CheckBox` is the field-correct accesskit 0.21.1 toggle-state role (the MT names
    /// `Role::ToggleButton`, which does not exist there); `set_toggled` exposes the on/off state and
    /// `Action::Click` is the toggle action. A node is emitted for every painted row that has a
    /// breakpoint (capped at [`MAX_ACCESSKIT_GUTTER_MARKERS`]) so the test can assert the state change.
    fn emit_breakpoint_nodes(
        &self,
        ui: &egui::Ui,
        visible_rows: &[usize],
        breakpoints: &BreakpointSet,
        config: &GutterConfig,
    ) {
        if !config.show_breakpoints {
            return;
        }
        let lines: Vec<usize> = visible_rows
            .iter()
            .copied()
            .filter(|&l| breakpoints.contains(l))
            .take(MAX_ACCESSKIT_GUTTER_MARKERS)
            .collect();
        for (slot, line) in lines.into_iter().enumerate() {
            let author = self.breakpoint_author_id(line);
            let node_id = self.breakpoint_node_id(slot, line);
            let value = format!("breakpoint on line {}", line + 1);
            ui.ctx().accesskit_node_builder(node_id, move |node| {
                // DEVIATION (API-correct): Role::ToggleButton does not exist in accesskit 0.21.1;
                // Role::CheckBox is the field-correct toggle-state role (AC asserts the author_id +
                // the toggled state, not the role string — same pattern as MT-003 TextCursor->Caret).
                node.set_role(accesskit::Role::CheckBox);
                node.set_author_id(author.clone());
                node.set_label("Code editor breakpoint".to_owned());
                node.set_value(value.clone());
                node.set_toggled(accesskit::Toggled::True); // a node is only emitted when set
                node.add_action(accesskit::Action::Click);
            });
        }
    }

    /// MT-054: emit the word-wrap toggle AccessKit node. `Role::Button` with a `Toggled` property
    /// reflecting the persisted `WrapConfig.enabled` and `Action::Click` (the swarm Press action; egui /
    /// accesskit 0.21 maps a button Press to `Action::Click` — the MT names `actions=[Press]`, the same
    /// documented deviation pattern the breakpoint node uses for its toggle action). Author_id is the
    /// contract-named `editor-wrap-toggle` (suffixed for instances). The value carries the on/off state so
    /// an agent can read it without dispatching. Always emitted (the toggle is always present), so the
    /// AccessKit-id test + the interactive-naming gate both see a named interactive node.
    fn emit_wrap_toggle_node(&self, ui: &egui::Ui) {
        let author = self.wrap_toggle_author_id();
        let node_id = self.wrap_toggle_node_id();
        let enabled = self.is_wrap_enabled();
        let value = if enabled {
            "word wrap on"
        } else {
            "word wrap off"
        }
        .to_owned();
        ui.ctx().accesskit_node_builder(node_id, move |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(author.clone());
            node.set_label("Toggle word wrap".to_owned());
            node.set_value(value.clone());
            node.set_toggled(if enabled {
                accesskit::Toggled::True
            } else {
                accesskit::Toggled::False
            });
            node.add_action(accesskit::Action::Click);
        });
    }

    /// Emit one `Role::Label` AccessKit node per PAINTED diagnostic line (value = the worst severity +
    /// the message), so a swarm agent can read a line's diagnostic by `code_editor_diagnostic_{line}`.
    /// `Role::Label` is the field-correct accesskit 0.21.1 static-text role (the MT names
    /// `Role::StaticText`, which does not exist there). One node per painted line that has at least one
    /// diagnostic (capped at [`MAX_ACCESSKIT_GUTTER_MARKERS`]).
    fn emit_diagnostic_nodes(
        &self,
        ui: &egui::Ui,
        visible_rows: &[usize],
        markers: &[GutterMarker],
        config: &GutterConfig,
    ) {
        if !config.show_diagnostics {
            return;
        }
        let mut emitted = 0usize;
        for &line in visible_rows {
            if emitted >= MAX_ACCESSKIT_GUTTER_MARKERS {
                break;
            }
            let line_msgs: Vec<String> = markers
                .iter()
                .filter(|m| m.line == line && matches!(m.kind, GutterMarkerKind::Diagnostic(_)))
                .map(|m| match &m.kind {
                    GutterMarkerKind::Diagnostic(sev) if m.message.is_empty() => {
                        sev.label().to_owned()
                    }
                    GutterMarkerKind::Diagnostic(sev) => format!("{}: {}", sev.label(), m.message),
                    _ => String::new(),
                })
                .collect();
            if line_msgs.is_empty() {
                continue;
            }
            let author = self.diagnostic_author_id(line);
            let node_id = self.diagnostic_node_id(emitted, line);
            let value = line_msgs.join("\n");
            ui.ctx().accesskit_node_builder(node_id, move |node| {
                node.set_role(accesskit::Role::Label);
                node.set_author_id(author.clone());
                node.set_label("Code editor diagnostic".to_owned());
                node.set_value(value.clone());
            });
            emitted += 1;
        }
    }

    /// The fixed `egui::Id` for the gutter strip Group node (default panel; instances hash the author_id).
    fn gutter_node_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: a single hand-assigned fixed id in the disjoint gutter band; never reused.
            unsafe { egui::Id::from_high_entropy_bits(PANEL_GUTTER_NODE_ID) }
        } else {
            egui::Id::new(self.gutter_author_id())
        }
    }

    /// The fixed `egui::Id` for breakpoint node `slot` (default panel uses the breakpoint band; instances
    /// hash the suffixed author_id — RISK-004).
    fn breakpoint_node_id(&self, slot: usize, line: usize) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: each slot maps to a distinct fixed id in the disjoint breakpoint band; never reused.
            unsafe { egui::Id::from_high_entropy_bits(PANEL_BREAKPOINT_NODE_ID_BASE + slot as u64) }
        } else {
            egui::Id::new(format!(
                "{CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX}{line}#{}",
                self.instance
            ))
        }
    }

    /// The fixed `egui::Id` for diagnostic node `slot` (default panel uses the diagnostic band; instances
    /// hash the suffixed author_id — RISK-004).
    fn diagnostic_node_id(&self, slot: usize, line: usize) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: each slot maps to a distinct fixed id in the disjoint diagnostic band; never reused.
            unsafe { egui::Id::from_high_entropy_bits(PANEL_DIAGNOSTIC_NODE_ID_BASE + slot as u64) }
        } else {
            egui::Id::new(format!(
                "{CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX}{line}#{}",
                self.instance
            ))
        }
    }

    /// Clip the sorted cached span list to the half-open byte window `[win_start, win_end)`, returning
    /// just the spans that can overlap it. The cache is sorted by start byte, so a binary search finds
    /// the first span that could reach into the window; from there a forward scan collects spans until
    /// one starts past the window end. This bounds per-frame span work to the visible window rather
    /// than the whole document (MT-002 step 3). Spans are cloned out so the cache lock is not held
    /// across the egui layout calls in `render_line`.
    fn spans_in_byte_window(&self, win_start: usize, win_end: usize) -> HighlightSpanWindow {
        if win_end <= win_start {
            return HighlightSpanWindow::default();
        }
        let _pending = self.poll_initial_highlight();
        if self.initial_highlight_status_value() == InitialHighlightStatus::Pending {
            let _poll_guard = self
                .initial_highlight_poll
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if self.initial_highlight_status_value() == InitialHighlightStatus::Pending {
                let version = self.buffer_version.load(Ordering::Acquire);
                let generation = self.initial_highlight_generation.load(Ordering::Acquire);
                let source = self
                    .initial_highlight_source
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .as_ref()
                    .filter(|(_, source_version, source_generation)| {
                        *source_version == version && *source_generation == generation
                    })
                    .map(|(source, _, _)| Arc::clone(source));
                if let Some(source) = source {
                    let spans = self
                        .highlighter
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .as_ref()
                        .map(|highlighter| {
                            highlighter.captures_for_current_range(
                                &source,
                                win_start.min(source.len())..win_end.min(source.len()),
                            )
                        })
                        .unwrap_or_default();
                    return HighlightSpanWindow::from_spans(spans);
                }
            }
        }
        let cache = self
            .highlight_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((spans, _)) = cache.as_ref() else {
            return HighlightSpanWindow::default();
        };
        HighlightSpanWindow::from_spans(spans.overlapping(win_start, win_end).cloned().collect())
    }

    fn code_static_label(ui: &mut egui::Ui, text: egui::RichText) -> egui::Response {
        let resp = ui.label(text);
        ui.ctx().accesskit_node_builder(resp.id, |node| {
            node.set_role(accesskit::Role::Label);
        });
        resp
    }

    /// Emit ONE static text label for a whole code row from a pre-built multi-section [`LayoutJob`]
    /// (per-run colors baked into the sections). PERF (MT-002 AC-002 frame budget): a code row was
    /// previously painted as a SEPARATE `code_static_label` (a `ui.label` widget + an AccessKit Label
    /// node) PER highlight run — up to ~8 widgets/AccessKit-nodes per row, times the ~45 rows painted per
    /// frame on a 100k-line file, was the dominant per-frame cost that blew the release frame budget.
    /// Collapsing the row into one galley (egui still caches the laid-out galley by the `LayoutJob`)
    /// leaves exactly ONE widget + ONE AccessKit Label node per row, with identical monospace glyph
    /// positions (contiguous, zero item-spacing) and identical per-run colors — the whole line's text is
    /// the node's accessible value, so `by_label("line N")` still resolves to exactly this one node.
    fn code_static_label_job(ui: &mut egui::Ui, job: egui::text::LayoutJob) -> egui::Response {
        let resp = ui.label(job);
        ui.ctx().accesskit_node_builder(resp.id, |node| {
            node.set_role(accesskit::Role::Label);
        });
        resp
    }

    /// WP-KERNEL-012 MT-078 (E13 RTL/bidi): paint ONE code-editor row whose base direction is RTL
    /// (Hebrew/Arabic string literal or comment) OR which carries a Tier-3 shaping limitation, using the
    /// SHARED `text_intl::bidi` pass (NOT a parallel one) so the code editor and rich editor agree on bidi.
    /// Returns `true` when it handled the row (RTL base or a limitation present), `false` when the line is
    /// the pure-LTR IDENTITY and the caller should keep the existing byte-for-byte LTR run path (AC6 / MC-3
    /// / RISK-2 — no regression to ordinary source lines).
    ///
    /// What it does for an RTL row (mirrors `block_renderer.rs`'s RTL right-anchor pattern within the
    /// `ui.label` code path): reorders the line into VISUAL order via UAX#9 and lays it out RIGHT-ALIGNED
    /// within the code text column (`Layout::right_to_left`, anchored to the column's right edge — the
    /// `ScrollArea` is `auto_shrink([false,false])`, so `available_width` is the full text column). The rope
    /// stays logical-order; this is render-time only. Per-run syntax colors are NOT split across the bidi
    /// boundary here (the honest order+alignment tier — the same single-format simplification the rich
    /// editor's RTL path documents); the row is painted in the default foreground color.
    ///
    /// AC5 / PROOF3 / MC-1: when the line contains Arabic/Indic content egui cannot cursive-shape, it paints
    /// a VISIBLE typed-limitation marker (a `⚠` glyph in the subtle/comment theme color, hover text carrying
    /// the limitation note + future-MT pointer) on the row, so Arabic in the CODE editor is NEVER silently
    /// broken. The marker also fires for an Arabic literal inside an otherwise-LTR line (base LTR, limitation
    /// present) — which is why this returns `true` on a limitation even when the base is LTR.
    fn render_rtl_or_limited_code_row(
        &self,
        ui: &mut egui::Ui,
        line_text: &str,
        mono: &egui::FontId,
        syntax: &HsSyntaxTokens,
    ) -> bool {
        // PERF FAST-PATH (MT-002 AC-002 frame budget): the bidi entry point below runs UAX#9
        // (`BidiInfo::new` twice — once for the base direction, once for `visual_runs`) AND allocates a
        // reordered `visual_text` String for EVERY visible line EVERY frame. That is pure waste for the
        // overwhelmingly common case — a pure-ASCII source line — which is ALWAYS LTR-base and can NEVER
        // carry an Arabic/Indic shaping limitation (both require code points >= U+0600). `str::is_ascii`
        // is a cheap byte scan (SIMD-friendly), so short-circuit it here: an ASCII line is definitionally
        // the "pure-LTR identity" case the full pass resolves to `false` anyway. Non-ASCII lines (rare in
        // code) still take the full, correct bidi path below — no behavioral change, only the redundant
        // per-frame allocation+analysis removed. This was the dominant per-row cost the release bench hit.
        if line_text.is_ascii() {
            return false;
        }
        // The code-editor bidi entry point lives in `code_editor/virtual_lines.rs` (the MT-078 contract's
        // named code-editor deliverable); it reuses the SHARED `text_intl::bidi` pass internally.
        let bidi = super::virtual_lines::code_line_bidi(line_text);
        // Pure-LTR identity (the overwhelmingly common source line): let the caller keep the existing
        // per-run colored LTR path unchanged. Only an RTL base OR a shaping limitation takes this path.
        if !bidi.base.is_rtl() && bidi.shaping_limitation.is_none() {
            return false;
        }

        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let default_color = syntax.punctuation;

            // AC5 / PROOF3 / MC-1 — the visible typed-limitation marker for Arabic/Indic (never silently
            // broken). `⚠` is a literal glyph string (NOT a Color32 — CONTROL-4 holds); the color is the
            // subtle/comment theme token. Hover surfaces the note + the future-MT pointer.
            let limitation_note = bidi
                .shaping_limitation
                .as_ref()
                .map(|lim| format!("{}\n{}", lim.note, lim.pointer));

            if bidi.base.is_rtl() {
                // RTL base (Hebrew/Arabic line): paint the VISUAL-order text RIGHT-ALIGNED. A
                // right-to-left layout places widgets from the column's right edge leftward, so the
                // reordered line anchors to the right edge (AC1/AC3 — right-aligned RTL code line).
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if let Some(note) = &limitation_note {
                        Self::code_static_label(
                            ui,
                            egui::RichText::new("⚠")
                                .font(mono.clone())
                                .color(syntax.comment),
                        )
                        .on_hover_text(note);
                    }
                    let visual = if bidi.visual_text.is_empty() {
                        " ".to_string()
                    } else {
                        bidi.visual_text.clone()
                    };
                    Self::code_static_label(
                        ui,
                        egui::RichText::new(visual)
                            .font(mono.clone())
                            .color(default_color),
                    );
                });
            } else {
                // LTR base but the line CONTAINS Arabic/Indic (e.g. `let s = "العربية";`): keep the
                // line left-to-right (its base is LTR) but still surface the visible limitation marker so
                // the Arabic content is not silently presented as shaped. The text is the logical line
                // (the LTR base reads left-to-right); only the limitation marker is added.
                Self::code_static_label(
                    ui,
                    egui::RichText::new(line_text)
                        .font(mono.clone())
                        .color(default_color),
                );
                if let Some(note) = &limitation_note {
                    // A small gap then the bare `⚠` marker (its accessible text is exactly "⚠" so swarm
                    // queries / the integration test can find it; spacing is layout, not glyph text).
                    ui.add_space(mono.size * 0.5);
                    Self::code_static_label(
                        ui,
                        egui::RichText::new("⚠")
                            .font(mono.clone())
                            .color(syntax.comment),
                    )
                    .on_hover_text(note);
                }
            }
        });
        // Same duplicate-label fix as the non-wrap `render_line`: the RTL/limited row is a STRUCTURAL
        // wrapper around the inner text label(s), not a second text label. `Role::Label` here made
        // accesskit derive the container name from its subtree text (the reordered VISUAL string), so an
        // RTL line surfaced the SAME visual-order string on BOTH the container and the inner label — two
        // nodes for one visible line, breaking `query_by_label(visual_text)` single-node queries. Use a
        // GenericContainer (no name derivation); the inner `code_static_label` carries the one text label.
        ui.ctx().accesskit_node_builder(row.response.id, |node| {
            node.set_role(accesskit::Role::GenericContainer);
        });
        true
    }

    /// Render one line as a sequence of theme-colored runs, splitting the line text at the highlight
    /// span boundaries that overlap it. `visible_spans` is the per-frame window-clipped span slice (so
    /// this is O(spans-in-window), not O(all-spans)). A line with no overlapping spans renders as plain
    /// foreground text. Byte->char conversions go through the buffer (RISK-002).
    ///
    /// MT-078 (E13 RTL/bidi): before the LTR run path, an RTL or limitation-bearing line is delegated to
    /// [`render_rtl_or_limited_code_row`] (reorder + right-align + the visible Arabic/Indic limitation
    /// marker). A pure-LTR line falls through to the EXACT existing per-run colored path (AC6 identity).
    fn render_line(
        &self,
        ui: &mut egui::Ui,
        line_idx: usize,
        visible_spans: &HighlightSpanWindow,
        syntax: &HsSyntaxTokens,
    ) {
        let (line_text_owned, line_start_byte) = self.with_buffer(|b| {
            (
                b.slice_to_string(line_idx..line_idx + 1),
                b.line_to_byte(line_idx).unwrap_or(0),
            )
        });
        // Strip the trailing newline so each visual line is one row (the layout adds the row break).
        let line_text = line_text_owned
            .strip_suffix('\n')
            .unwrap_or(&line_text_owned);
        let line_end_byte = line_start_byte + line_text.len();

        let mono = self.mono_font();
        // MT-078: an RTL line (Hebrew/Arabic literal/comment) is reordered + right-aligned, and an
        // Arabic/Indic line surfaces the visible typed-limitation marker — via the shared bidi pass.
        // A pure-LTR line returns `false` and falls through to the EXACT existing per-run path (AC6).
        if self.render_rtl_or_limited_code_row(ui, line_text, &mono, syntax) {
            return;
        }

        // Spans overlapping THIS line, clipped to the line's byte window from the already
        // window-clipped frame slice. The prefix max-end index preserves long parent spans that start
        // above the row even when shorter child spans ended before it.
        let mut runs: Vec<(std::ops::Range<usize>, HighlightScope)> = Vec::new();
        for span in visible_spans.overlapping(line_start_byte, line_end_byte) {
            let s = span.byte_range.start.max(line_start_byte);
            let e = span.byte_range.end.min(line_end_byte);
            if s < e {
                runs.push((s..e, span.scope));
            }
        }
        runs.sort_by_key(|(r, _)| r.start);

        let default_color = syntax.punctuation;

        // Helper to slice a [start,end) byte window of the line into a &str safely (RISK-002:
        // respect char boundaries; fall back to empty on a bad boundary). Returns a BORROWED slice of
        // `line_text` (not an owned String): `LayoutJob::append` copies the text into the job's own buffer,
        // so there is no need to allocate a String per run — this removes ~N-runs heap allocations per
        // painted row per frame from the MT-002 frame-budget path.
        let line_slice = |start: usize, end: usize| -> &str {
            let rel_start = start.saturating_sub(line_start_byte);
            let rel_end = end.saturating_sub(line_start_byte);
            if rel_start >= rel_end || rel_end > line_text.len() {
                return "";
            }
            let mut a = rel_start;
            while a < line_text.len() && !line_text.is_char_boundary(a) {
                a += 1;
            }
            let mut b = rel_end.min(line_text.len());
            while b < line_text.len() && !line_text.is_char_boundary(b) {
                b += 1;
            }
            if a >= b {
                return "";
            }
            &line_text[a..b]
        };

        // Build ONE colored `LayoutJob` for the whole row (per-run sections) instead of a `ui.label`
        // widget per run — see `code_static_label_job` for the frame-budget rationale. The append order +
        // slices are IDENTICAL to the previous per-label path, so glyphs and colors are byte-for-byte the
        // same; only the widget/AccessKit-node count per row collapses from ~N-runs to one.
        let mut job = egui::text::LayoutJob::default();
        let append = |text: &str, color: egui::Color32, job: &mut egui::text::LayoutJob| {
            if !text.is_empty() {
                job.append(
                    text,
                    0.0,
                    egui::TextFormat {
                        font_id: mono.clone(),
                        color,
                        ..Default::default()
                    },
                );
            }
        };
        let mut cursor = line_start_byte;
        for (range, scope) in &runs {
            if range.start > cursor {
                // Plain (un-highlighted) gap before this run.
                append(line_slice(cursor, range.start), default_color, &mut job);
            }
            append(
                line_slice(range.start, range.end),
                self.resolve_highlight_color(*scope, syntax),
                &mut job,
            );
            cursor = cursor.max(range.end);
        }
        // Trailing plain text after the last run.
        if cursor < line_end_byte {
            append(line_slice(cursor, line_end_byte), default_color, &mut job);
        }
        // Empty line: emit a zero-width spacer so the row still occupies a line height.
        if runs.is_empty() && line_text.is_empty() {
            append(" ", default_color, &mut job);
        }

        // Wrap the single row label in a `ui.horizontal` whose `interact_size.y` floor pins the row to
        // EXACTLY `line_height`. This wrapper is LOAD-BEARING for the MT-054 one-unit row-pitch invariant:
        // a bare `ui.label` advances the vertical cursor by the galley's PIXEL-ROUNDED height (15.000)
        // rather than the geometry unit `line_height` (15.125), which diverges the painted pitch from the
        // gutter/overlay/decoration stride. Keeping the wrapper preserves the exact pitch.
        let row = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            Self::code_static_label_job(ui, job);
        });
        // The row is a STRUCTURAL wrapper around the single text label, NOT a second text label. Marking
        // it `Role::Label` made accesskit derive its accessible name from the subtree text
        // (name-from-contents), so a plain-text line surfaced the SAME "line N" string on BOTH the
        // container node and the inner text label — two nodes for one visible line, which broke every
        // `get_by_label("line N")` / `query_by_label("line N")` single-node query (the inner
        // `code_static_label_job` already carries the row's text as the one addressable Label). Use a
        // GenericContainer role: a structural grouping that does NOT derive a name, so each painted line
        // exposes exactly ONE "line N" text label (its glyph row), with the container as its parent.
        ui.ctx().accesskit_node_builder(row.response.id, |node| {
            node.set_role(accesskit::Role::GenericContainer);
        });
    }

    // ── MT-004 find-match highlight overlay ────────────────────────────────────────────────────────────

    /// Paint a translucent rect over every find match in the painted row window (AC-005): yellow for an
    /// ordinary match, orange for the CURRENT match. A no-op when the find bar is closed (`find_state`
    /// is `None`) so AC-006 holds — closing the bar removes every highlight on the next frame. Only
    /// matches whose line falls inside `geometry.first_line..end_line` are drawn (implementation note 2:
    /// off-screen matches are skipped for performance on large files). A match that spans columns on one
    /// line draws one rect from its start col to its end col; the rare multi-line regex match draws one
    /// rect per covered line. Column->x / line->y reuse the SAME units as the cursor overlay (the
    /// MT-002 sans-spacing line_height + monospace glyph_width — implementation note: positioning unit
    /// dependency from MT-002 AC-007).
    fn paint_match_highlights(
        &self,
        ui: &egui::Ui,
        geometry: &RowGeometry,
        glyph_width: f32,
        painted_lines: &[usize],
        wrap_rows: Option<&[VisualRow]>,
    ) {
        let state = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = state.as_ref() else {
            return; // bar closed -> no highlights (AC-006)
        };
        if state.matches.is_empty() {
            return;
        }
        let painter = ui.painter();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let x_for = |col: usize| geometry.left + col as f32 * glyph_width;
        // FOLD-AWARE row y (Wave-B fix): a line's y is its PAINTED row offset — `None` for a
        // fold-hidden or off-window line (no rect painted), never `line - first_line` arithmetic.
        let y_for = |line: usize| -> Option<f32> {
            Self::painted_row_offset(painted_lines, line)
                .map(|row| geometry.top + row as f32 * geometry.line_height)
        };

        for (idx, m) in state.matches.iter().enumerate() {
            let color = if idx == state.current_match {
                CURRENT_MATCH_HIGHLIGHT_COLOR
            } else {
                MATCH_HIGHLIGHT_COLOR
            };
            // MT-054 Task-B: under WORD WRAP the match spans visual rows, so map the byte range through
            // the painted visual rows (one rect per visual row it covers) instead of the non-wrap
            // per-logical-line mapping below.
            if let Some(rows) = wrap_rows {
                for rect in Self::wrap_overlay_rects(
                    rows,
                    &buffer,
                    m.byte_range.clone(),
                    geometry,
                    glyph_width,
                ) {
                    painter.rect_filled(rect, 0.0, color);
                }
                continue;
            }
            // The match's start/end (line, col). A match is usually single-line; a multi-line regex
            // match is handled by drawing one rect per covered line (start col on the first line, end
            // col on the last, whole content width between).
            let (start_line, start_col) = byte_to_line_col(m.byte_range.start, &buffer);
            let (end_match_line, end_col) = byte_to_line_col(m.byte_range.end, &buffer);
            for line in start_line..=end_match_line {
                let Some(y0) = y_for(line) else {
                    continue; // off-screen or fold-hidden row (implementation note 2)
                };
                let line_start_col = if line == start_line { start_col } else { 0 };
                let line_end_col = if line == end_match_line {
                    end_col
                } else {
                    // A continuation row of a multi-line match extends to the line content end.
                    let (_, content_end_col) =
                        byte_to_line_col(line_col_to_byte(line, usize::MAX, &buffer), &buffer);
                    content_end_col.max(line_start_col + 1)
                };
                // Never a zero-width rect: a single empty match would not show, but the engine never
                // returns empty matches (the pattern is non-empty). Guard anyway so an oddity is visible.
                let visual_end_col = line_end_col.max(line_start_col + 1);
                let x0 = x_for(line_start_col);
                let x1 = x_for(visual_end_col);
                let rect = egui::Rect::from_min_max(
                    egui::pos2(x0, y0),
                    egui::pos2(x1, y0 + geometry.line_height),
                );
                painter.rect_filled(rect, 0.0, color);
            }
        }
    }

    /// MT-071: paint the render-whitespace glyphs (a middot `·` for each space, an arrow `→` for each
    /// tab) over the painted rows when the doc-model `render_whitespace` flag is on. Restricted to the
    /// on-screen buffer window `geometry.first_line..end_line` so it stays cheap on a large file (the
    /// same window discipline the find-match + cursor overlays use). The glyph color is the theme's
    /// subtle `punctuation` token (no hex literal — the theme guard), one tone below the code text so the
    /// markers read as faint guides, not content. Whitespace INSIDE the line (not just leading) is
    /// marked, matching VS Code's "all" render mode.
    fn paint_whitespace_glyphs(
        &self,
        ui: &egui::Ui,
        geometry: &RowGeometry,
        glyph_width: f32,
        painted_lines: &[usize],
        syntax: &HsSyntaxTokens,
        wrap_rows: Option<&[VisualRow]>,
    ) {
        let painter = ui.painter();
        let color = syntax.punctuation;
        let font = self.mono_font();
        let x_for = |col: usize| geometry.left + col as f32 * glyph_width;
        // Read only the PAINTED rows (never the whole rope — RISK-003 window discipline). FOLD-AWARE
        // (Wave-B fix): iterate the painted row list itself, so a fold-hidden line's whitespace is
        // never marked and each row's y is its real painted offset.
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        // MT-035: honor the 3-way render-whitespace MODE (fixing the old Boundary-vs-All lossiness). In
        // `Boundary` mode a SINGLE space between two visible characters is NOT marked (VS Code parity),
        // while leading/trailing spaces, runs of 2+ spaces, and every tab are still marked. `All` marks
        // every space + tab. `None` never reaches here (the draw-gate `render_whitespace()` is false).
        let boundary = matches!(
            self.render_whitespace_mode(),
            crate::workspace_settings::RenderWhitespaceMode::Boundary
        );
        // Mark one row of whitespace at `row_y` over the characters of `text` (a `\n`/`\r` is never
        // marked). Shared by the non-wrap (whole logical line) and wrap (per-fragment) paths.
        let mark_row = |row_y: f32, text: &str| {
            let chars: Vec<char> = text.chars().collect();
            let is_visible = |c: char| c != ' ' && c != '\t' && c != '\n' && c != '\r';
            let mut col = 0usize;
            for (i, &ch) in chars.iter().enumerate() {
                match ch {
                    ' ' => {
                        // Boundary mode: skip a lone space bordered by visible chars on BOTH sides.
                        let skip = boundary
                            && i > 0
                            && is_visible(chars[i - 1])
                            && chars.get(i + 1).copied().map(is_visible).unwrap_or(false);
                        if !skip {
                            let center = egui::pos2(
                                x_for(col) + glyph_width * 0.5,
                                row_y + geometry.line_height * 0.5,
                            );
                            painter.text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                "·",
                                font.clone(),
                                color,
                            );
                        }
                        col += 1;
                    }
                    '\t' => {
                        let center = egui::pos2(
                            x_for(col) + glyph_width * 0.5,
                            row_y + geometry.line_height * 0.5,
                        );
                        painter.text(
                            center,
                            egui::Align2::CENTER_CENTER,
                            "→",
                            font.clone(),
                            color,
                        );
                        col += 1;
                    }
                    '\n' | '\r' => {} // do not mark the trailing newline
                    _ => {
                        col += 1;
                    }
                }
            }
        };
        match wrap_rows {
            None => {
                for (row_offset, &line) in painted_lines.iter().enumerate() {
                    let row_y = geometry.top + row_offset as f32 * geometry.line_height;
                    let line_text = buffer.slice_to_string(line..line + 1);
                    mark_row(row_y, &line_text);
                }
            }
            // MT-054 Task-B: under WORD WRAP each painted row is one wrap fragment; mark whitespace per
            // fragment so continuation rows carry their own middots/arrows at the correct visual row.
            Some(rows) => {
                for (idx, row) in rows.iter().enumerate() {
                    let row_y = geometry.top + idx as f32 * geometry.line_height;
                    let frag = buffer.byte_slice_to_string(row.byte_start..row.byte_end);
                    mark_row(row_y, &frag);
                }
            }
        }
    }

    // ── MT-003 overlay + AccessKit + input ───────────────────────────────────────────────────────────

    /// Paint every caret (a 2px vertical bar) and every selection (a semi-transparent rect) over the
    /// painted rows, restricted to the PAINTED row window so a caret only draws where its glyph is
    /// actually rendered (MT-003 step 6). A selection that spans multiple lines is drawn as one rect
    /// per PAINTED line in the span (so a box/column selection naturally shows one rect per row, and a
    /// fold-hidden line draws nothing). Column->x uses `glyph_width`; line->y is the line's painted row
    /// offset in `painted_lines` times `line_height` (the MT-054 fold-aware unit — the MT-003-era
    /// `line - first_line` contiguity bug is fixed here per the Wave-B audit).
    fn paint_cursor_overlay(
        &self,
        ui: &egui::Ui,
        geometry: &RowGeometry,
        glyph_width: f32,
        painted_lines: &[usize],
        syntax: &HsSyntaxTokens,
        wrap_rows: Option<&[VisualRow]>,
    ) {
        let painter = ui.painter();
        // Caret color: the editor foreground (theme-sourced, never a hex literal). Selection overlay is
        // the MT-named cornflower-blue at low alpha — a fixed selection-highlight tint that is NOT a
        // syntax-token color (it is a UI affordance, like egui's own selection bg), so it is the one
        // place the MT contract specifies an explicit RGBA. Kept exactly as the contract names it.
        let caret_color = syntax.punctuation;
        let selection_color = egui::Color32::from_rgba_unmultiplied(100, 149, 237, 80);

        let cursors = self
            .cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());

        let x_for = |col: usize| geometry.left + col as f32 * glyph_width;
        // FOLD-AWARE row y (Wave-B fix): a line's y is its PAINTED row offset — `None` for a
        // fold-hidden or off-window line (nothing drawn there).
        let y_for = |line: usize| -> Option<f32> {
            Self::painted_row_offset(painted_lines, line)
                .map(|row| geometry.top + row as f32 * geometry.line_height)
        };

        for cursor in cursors.cursors() {
            // Draw the SELECTION (if any) first, then the caret on top, so the caret stays visible.
            if cursor.is_selection() {
                let range = cursor.range();
                match wrap_rows {
                    None => {
                        let (start_line, start_col) = byte_to_line_col(range.start, &buffer);
                        let (end_line_sel, end_col) = byte_to_line_col(range.end, &buffer);
                        for line in start_line..=end_line_sel {
                            let Some(y0) = y_for(line) else {
                                continue; // off-screen or fold-hidden row
                            };
                            // Column span on THIS line: from the selection start col (or 0 if the line is
                            // not the first) to the end col (or the line's content end if not the last).
                            let line_start_col = if line == start_line { start_col } else { 0 };
                            let line_end_col = if line == end_line_sel {
                                end_col
                            } else {
                                // Whole-line selection rows extend to the line's char length.
                                let (_, content_end_col) = byte_to_line_col(
                                    line_col_to_byte(line, usize::MAX, &buffer),
                                    &buffer,
                                );
                                content_end_col.max(line_start_col + 1) // at least 1 col wide
                            };
                            if line_end_col <= line_start_col {
                                continue;
                            }
                            let x0 = x_for(line_start_col);
                            let x1 = x_for(line_end_col);
                            let rect = egui::Rect::from_min_max(
                                egui::pos2(x0, y0),
                                egui::pos2(x1, y0 + geometry.line_height),
                            );
                            painter.rect_filled(rect, 0.0, selection_color);
                        }
                    }
                    // MT-054 Task-B: WORD WRAP selection — one rect per visual row the range covers.
                    Some(rows) => {
                        for rect in
                            Self::wrap_overlay_rects(rows, &buffer, range, geometry, glyph_width)
                        {
                            painter.rect_filled(rect, 0.0, selection_color);
                        }
                    }
                }
            }
            // Draw the caret (the moving head) as a 2px vertical bar. Non-wrap maps the head's (line,col)
            // through the fold-aware painted rows; wrap maps the head byte through the painted visual rows
            // (MT-054 Task-B) so the caret sits on the correct WRAPPED row.
            let caret_xy = match wrap_rows {
                None => {
                    let (head_line, head_col) = byte_to_line_col(cursor.head, &buffer);
                    y_for(head_line).map(|y| (x_for(head_col), y))
                }
                Some(rows) => {
                    Self::wrap_overlay_pos(rows, &buffer, cursor.head, geometry, glyph_width)
                }
            };
            if let Some((x, y)) = caret_xy {
                let caret = egui::Rect::from_min_max(
                    egui::pos2(x, y),
                    egui::pos2(x + 2.0, y + geometry.line_height),
                );
                painter.rect_filled(caret, 0.0, caret_color);
            }
        }

        // WP-KERNEL-012 MT-076 (E13 IME inline preedit / AC2 + AC4): if an IME composition is in progress,
        // paint the preedit text UNDERLINED at the PRIMARY caret (overlay-only — never in the buffer,
        // RISK-1 / MC-1) and report the IME caret rect to the OS so the candidate window anchors at the
        // caret, not the window origin (RISK-2 / MC-2). The preedit is laid out in the SAME monospace
        // FontId the rows use, underlined, over a subtle theme-tinted background (no hex literal — the
        // selection tint reuse / theme tokens). A no-op when the composition is empty or the caret is
        // scrolled off the painted window.
        let preedit = self
            .preedit
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if !preedit.is_empty() {
            let primary = cursors.primary();
            // MT-054 Task-B: anchor the IME preedit at the primary caret in BOTH modes (wrap maps the
            // head byte through the painted visual rows, like the caret above).
            let preedit_xy = match wrap_rows {
                None => {
                    let (head_line, head_col) = byte_to_line_col(primary.head, &buffer);
                    y_for(head_line).map(|y| (x_for(head_col), y))
                }
                Some(rows) => {
                    Self::wrap_overlay_pos(rows, &buffer, primary.head, geometry, glyph_width)
                }
            };
            if let Some((x, y)) = preedit_xy {
                // Lay out the preedit in the editor's monospace font so it matches the code glyphs.
                let font = self.mono_font();
                let galley = painter.layout_no_wrap(preedit.clone(), font, caret_color);
                let run_w = galley.rect.width().max(glyph_width);
                let run_h = geometry.line_height;
                let origin = egui::pos2(x, y);
                let overall_rect = egui::Rect::from_min_size(origin, egui::vec2(run_w, run_h));
                // Subtle in-progress background (the same low-alpha cornflower selection tint the overlay
                // already uses as a UI affordance, not a syntax color) so the composing run is distinct.
                painter.rect_filled(overall_rect, 1.0, selection_color);
                painter.galley(origin, std::sync::Arc::clone(&galley), caret_color);
                // Underline the composing run (a 1px line in the caret color at the row baseline).
                let underline_y = y + run_h - 1.0;
                painter.line_segment(
                    [
                        egui::pos2(x, underline_y),
                        egui::pos2(x + run_w, underline_y),
                    ],
                    egui::Stroke::new(1.0, caret_color),
                );
                // The composition caret sits at the END of the preedit run (egui 0.33 Preedit carries no
                // cursor range — the field-correct position).
                let caret_x = x + run_w;
                let cursor_rect =
                    egui::Rect::from_min_size(egui::pos2(caret_x, y), egui::vec2(2.0, run_h));
                // AC4: report the IME caret rect so the OS candidate list anchors at the caret.
                ui.ctx().output_mut(|o| {
                    o.ime = Some(egui::output::IMEOutput {
                        rect: overall_rect,
                        cursor_rect,
                    });
                });
            }
        }
    }

    /// Emit one `Role::Caret` AccessKit node per cursor (capped at [`MAX_ACCESSKIT_CURSORS`] —
    /// RISK-004 / MC-004) so a swarm agent can find each caret by `code_editor_cursor_{n}` (n = sorted
    /// index). Each node carries the cursor's `(line, col)` head position in its value field. The nodes
    /// are emitted onto fixed `egui::Id`s in the cursor band (default panel) so their `NodeId`s are
    /// stable across frames; they are children of the current (text) scope's `Ui`. (The MT contract
    /// named `Role::TextCursor`, which does not exist in accesskit 0.21 — `Role::Caret` is the
    /// field-correct equivalent; the body documents the deviation in full.)
    fn emit_cursor_nodes(&self, ui: &egui::Ui) {
        let cursors = self
            .cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let count = cursors.len().min(MAX_ACCESSKIT_CURSORS);
        for (n, cursor) in cursors.cursors().iter().take(count).enumerate() {
            let (line, col) = byte_to_line_col(cursor.head, &buffer);
            let author = if self.instance.is_empty() {
                format!("{CODE_EDITOR_CURSOR_AUTHOR_PREFIX}{n}")
            } else {
                format!("{CODE_EDITOR_CURSOR_AUTHOR_PREFIX}{n}#{}", self.instance)
            };
            let value = format!("line {line} col {col}");
            let node_id = self.cursor_node_id(n);
            ui.ctx().accesskit_node_builder(node_id, move |node| {
                // DEVIATION (API-correct): the MT contract names `Role::TextCursor`, which does NOT
                // exist in accesskit 0.21 (the version pinned by eframe 0.33). `Role::Caret` is the
                // field-correct accesskit role for a text caret/cursor — the same concept the contract
                // intends. AC-004/PT-004 assert the `code_editor_cursor_{n}` author_id, not the role
                // string, so this satisfies the AC while using the real API. (Rubric: prescribed API
                // wrong for the real environment -> use the field-correct equivalent + document it.)
                node.set_role(accesskit::Role::Caret);
                node.set_author_id(author.clone());
                node.set_label("Code editor cursor".to_owned());
                node.set_value(value.clone());
            });
        }
    }

    /// The fixed `egui::Id` for cursor node `n` (default panel uses the cursor band; instances hash the
    /// suffixed author_id so two panels never share a cursor id — RISK-004).
    fn cursor_node_id(&self, n: usize) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: each `n` maps to a distinct fixed slot in the disjoint cursor band; never reused.
            unsafe { egui::Id::from_high_entropy_bits(PANEL_CURSOR_NODE_ID_BASE + n as u64) }
        } else {
            egui::Id::new(format!(
                "{CODE_EDITOR_CURSOR_AUTHOR_PREFIX}{n}#{}",
                self.instance
            ))
        }
    }

    /// The SCREEN position of the top-left of `(line, col)` from the most recent painted frame, or
    /// `None` if that line is outside the painted window (or no frame has rendered). The deterministic
    /// inverse of [`pointer_to_byte`](Self::pointer_to_byte): a kittest test computes the exact pixel to
    /// inject an Alt+Click at so the click lands on a known cell (AC-004). Adds half a glyph so the
    /// click lands inside the cell, not on its left edge.
    pub fn screen_pos_for_line_col(
        &self,
        line: usize,
        col: usize,
        glyph_width: f32,
    ) -> Option<egui::Pos2> {
        let g = (*self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()))?;
        // FOLD-AWARE painted row offset (MT-054 Wave-B fix — the same `line - first_line` contiguity
        // bug the overlays had): find `line` among the painted window's buffer lines, so the returned
        // pixel is the row egui ACTUALLY painted the line at, bounded above AND below (a fold-hidden
        // or off-window line returns `None` — there is no on-screen pixel for it).
        let visible_range = self.last_visible_range();
        let row_offset = {
            let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            (visible_range.start..visible_range.end)
                .position(|v| set.visible_line_to_buffer_line(v) == line)?
        };
        let x = g.left + col as f32 * glyph_width + glyph_width * 0.25;
        let y = g.top + row_offset as f32 * g.line_height + g.line_height * 0.5;
        Some(egui::pos2(x, y))
    }

    /// The cached monospace glyph width measured on the last `show` (px), or `None` before the first
    /// frame. Test/support observability seam: lets existing kittest proofs compute click pixels with the
    /// SAME width the overlay uses.
    #[doc(hidden)]
    pub fn measured_glyph_width(&self) -> Option<f32> {
        *self
            .glyph_width_px
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    /// Map a screen position to a buffer byte offset using the captured row geometry (MT-003 pointer
    /// hit-testing). Returns `None` if no geometry is captured yet (no frame painted). Clamps the
    /// column to the clicked line's length (RISK-002) so a click past the line end lands at the line
    /// end, never past it.
    fn pointer_to_byte(
        &self,
        pos: egui::Pos2,
        glyph_width: f32,
        total_lines: usize,
    ) -> Option<usize> {
        let geometry = (*self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()))?;
        if geometry.line_height <= 0.0 || glyph_width <= 0.0 {
            return None;
        }
        // FOLD-AWARE (MT-054 Wave-B fix): the clicked ROW maps to a buffer line through the SAME
        // fold-filtered visible map the painter used — `first_line + row` lands on a HIDDEN line when
        // a fold is inside the window. The clicked row's visible index is the painted window's start
        // plus the row offset; the fold map resolves it to the real buffer line (clamping past the
        // visible document to its last line, then to the buffer).
        let rel_y = (pos.y - geometry.top).max(0.0);
        let row = (rel_y / geometry.line_height).floor() as usize;
        let visible_idx = self.last_visible_range().start + row;
        let line = {
            let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            set.visible_line_to_buffer_line(visible_idx)
        }
        .min(total_lines.saturating_sub(1));
        // Column = round((x - left) / glyph_width), clamped to >= 0; line_col_to_byte clamps to the
        // line length.
        let rel_x = (pos.x - geometry.left).max(0.0);
        let col = (rel_x / glyph_width).round() as usize;
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        Some(line_col_to_byte(line, col, &buffer))
    }

    // ── MT-010 Monaco-parity keymap: the SINGLE key dispatch authority ────────────────────────────

    /// A snapshot clone of the active keymap (VS Code defaults + operator overrides). For tests + the
    /// command-palette/manual hint surface. The keymap is cheap to clone (a small binding Vec + a map).
    pub fn keymap(&self) -> Keymap {
        self.keymap
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Replace the active keymap (e.g. after an operator override reload or a programmatic rebind), bump
    /// the keymap version so the cached AccessKit command nodes + chord hints rebuild (RISK-002), and
    /// clear any pending two-chord prefix (a keymap swap invalidates an in-flight prefix).
    pub fn set_keymap(&self, keymap: Keymap) {
        *self.keymap.lock().unwrap_or_else(|e| e.into_inner()) = keymap;
        self.keymap_version.fetch_add(1, Ordering::Relaxed);
        *self.pending_chord.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Reload the keymap from `~/.handshake/keymap.json` settings (AC-007) and apply it. Used by the
    /// shell's "Configure keybindings" reload path and the hot-reload poll. A load error keeps the
    /// current keymap (logged) rather than reverting to bare defaults mid-session.
    pub fn reload_keymap_from_settings(&self, settings: &KeymapSettings) {
        self.set_keymap(Keymap::from_settings(settings));
    }

    /// Inject the command-palette dispatch channel (implementation note: `OpenCommandPalette` routes to
    /// the SAME WP-011 command palette, not a second one). The host clones a `Sender<CodeEditorAction>`
    /// it drains into the shell command bus. The same per-component injection pattern `set_runtime` uses.
    pub fn set_command_palette_sender(
        &self,
        tx: mpsc::Sender<CodeEditorHostCommand>,
        document_id: impl Into<String>,
    ) {
        *self
            .command_palette_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((tx, document_id.into()));
    }

    /// The current keymap version (bumped on every keymap swap). For tests + the AccessKit command-node
    /// cache key.
    pub fn keymap_version(&self) -> u64 {
        self.keymap_version.load(Ordering::Relaxed)
    }

    /// Materialize the operator keybinding override file at `~/.handshake/keymap.json` if it does not
    /// already exist (the 'Configure keybindings' button calls this — implementation note: "for now,
    /// just open the file"; we ensure it EXISTS for the operator to edit, focus-safely, instead of
    /// launching an external editor that would steal focus — HBR-QUIET). Writes an empty (no-override)
    /// settings document so the file is valid JSON the operator can extend. Returns the path written, or
    /// `None` when the home directory is unresolvable. An existing file is left untouched (the operator's
    /// edits are preserved — never clobbered).
    pub fn ensure_keymap_file_exists(&self) -> Option<std::path::PathBuf> {
        let path = self.keymap_file_path.clone()?;
        if !path.exists() {
            if let Err(e) = KeymapSettings::save_to_file(&path, &KeymapSettings::default()) {
                tracing::warn!(error = %e, "could not create keymap.json");
                return None;
            }
            tracing::info!(path = %path.display(), "created keymap.json for editing");
        }
        Some(path)
    }

    /// True while a two-chord prefix (e.g. Ctrl+K) is pending its second chord (RISK-001 surface for a
    /// test / a status hint).
    pub fn is_chord_pending(&self) -> bool {
        self.pending_chord
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// Test hook: back-date a pending two-chord prefix by `elapsed` so the next `process_keymap`'s
    /// timeout branch (`seen_at.elapsed() >= TWO_CHORD_TIMEOUT`) fires WITHOUT a real wall-clock sleep.
    /// Returns `true` if a prefix was pending and was aged. Used only by the keymap timeout test to
    /// exercise the REAL clear branch deterministically (no 3-second test sleep). A no-op when no prefix
    /// is pending.
    pub fn age_pending_chord_for_test(&self, elapsed: std::time::Duration) -> bool {
        let mut pending = self.pending_chord.lock().unwrap_or_else(|e| e.into_inner());
        if let Some((chord, seen_at)) = *pending {
            let aged = seen_at.checked_sub(elapsed).unwrap_or(seen_at);
            *pending = Some((chord, aged));
            true
        } else {
            false
        }
    }

    /// The stable AccessKit author_id for the command node of `action`: `code_editor_cmd_{name}` with
    /// the instance suffix (RISK-004). This is what a swarm agent / MCP tool addresses to dispatch the
    /// command without simulating a keystroke (AC-005 / HBR-SWARM).
    pub fn command_author_id(&self, action: CodeEditorAction) -> String {
        self.suffixed(&format!(
            "{CODE_EDITOR_COMMAND_AUTHOR_PREFIX}{}",
            action.name()
        ))
    }

    /// Dispatch an editor command by its AccessKit `code_editor_cmd_*` author_id — the path a swarm
    /// agent (via AccessKit `Action::Click` on the hidden node) or an MCP swarm tool takes to drive the
    /// editor WITHOUT simulating a keystroke (AC-005 / HBR-SWARM). Resolves the author_id to its
    /// [`CodeEditorAction`] through the cached command-node descriptors and dispatches it. Returns the
    /// dispatched action, or `None` for an unknown author_id (so a bad id is a no-op, not a panic).
    pub fn dispatch_command_by_author_id(&self, author_id: &str) -> Option<CodeEditorAction> {
        self.ensure_command_nodes();
        let action = {
            let cache = self
                .command_node_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            cache
                .as_ref()
                .and_then(|(_, descs)| descs.iter().find(|d| d.author_id == author_id))
                .map(|d| d.action)
        };
        if let Some(action) = action {
            self.dispatch_action(action);
        }
        action
    }

    // ── WP-KERNEL-012 MT-041 (E7): consolidated editor-action AccessKit surface ──────────────────────

    /// Install the shared [`EditorActionRegistry`] this code pane registers its canonical
    /// `editor.code.<action>` nodes into (MT-041). `instance_index` is the pane's stable 0-based index
    /// (0 for a single pane; >0 for a second+ code pane so the author_ids suffix `.<idx>` —
    /// RISK-041-05). After install, every `show` syncs + emits + consumes through this registry. Idempotent.
    pub fn install_editor_action_registry(
        &self,
        registry: Arc<Mutex<EditorActionRegistry>>,
        instance_index: usize,
    ) {
        let handle = {
            let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.register(EditorPaneType::Code, instance_index)
        };
        *self
            .editor_action_wiring
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(EditorActionWiring { registry, handle });
    }

    /// Install the shared registry under a complete stable document identity. File-backed MT008
    /// panels use this path so canonical action ids cannot collide through a truncated numeric hash.
    pub fn install_editor_action_registry_named(
        &self,
        registry: Arc<Mutex<EditorActionRegistry>>,
        instance_key: impl Into<String>,
    ) {
        let handle = {
            let mut reg = registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.register_named(EditorPaneType::Code, instance_key)
        };
        *self
            .editor_action_wiring
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(EditorActionWiring { registry, handle });
    }

    /// Detach this pane from the shared editor-action registry and remove its complete namespace.
    /// Called only after the owning document has truly closed.
    pub fn uninstall_editor_action_registry(&self) {
        let wiring = self
            .editor_action_wiring
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take();
        if let Some(wiring) = wiring {
            wiring
                .registry
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove_registration(&wiring.handle);
        }
    }

    /// Whether this panel currently owns a live editor-action registry namespace.
    pub fn has_editor_action_registry(&self) -> bool {
        self.editor_action_wiring
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .is_some()
    }

    /// Sync this pane's canonical `editor.code.<action>` nodes into the installed registry, emit them
    /// into the live AccessKit tree, and CONSUME any swarm `Action::Click` dispatched at them this frame
    /// (routing each to the real editor action it aliases). Called from [`show`](Self::show) when a
    /// registry is installed; a no-op when none is. Returns the canonical author_ids dispatched this
    /// frame (so a test can assert the dispatch reached the editor — RISK-041-04 / CTRL-041-04).
    ///
    /// CONSOLIDATION (anti-duplication): the nodes here are the ONE swarm-facing surface; they alias the
    /// existing `code_editor_cmd_*` / find-bar dispatch paths rather than re-minting parallel nodes
    /// (IN-041-08). A find option toggle's `checked` state is read from the live `find_state` so a
    /// ToggleButton never reports stale state (RISK-041-03 / CTRL-041-03).
    pub fn sync_editor_actions(&self, ui: &egui::Ui) -> Vec<String> {
        let wiring = self
            .editor_action_wiring
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some(wiring) = wiring.as_ref() else {
            return Vec::new();
        };
        let handle = wiring.handle.clone();
        let find_state = self.find_state();
        let find_open = find_state.is_some();
        let multi_cursor = self.cursor_count() > 1;
        // 1) Register/refresh every catalog node with its live state.
        {
            use crate::accessibility::editor_action_registry::AxRole;
            let mut reg = wiring.registry.lock().unwrap_or_else(|e| e.into_inner());
            for entry in CODE_ACTION_CATALOG {
                let author_id = handle.author_id(entry.action_id);
                let state =
                    self.code_action_state(entry, find_open, find_state.as_ref(), multi_cursor);
                reg.upsert(author_id, entry.role, entry.label, state);
            }
            // AC-041-04: a `editor.code.find-panel` node appears in the tree ONLY while the find panel is
            // open (its backing surface is the live find bar — `code_editor_find_bar`). Present-only (no
            // dispatch); a swarm agent reads it to confirm `find-open` took effect. Absent when closed.
            reg.upsert(
                handle.author_id("find-panel"),
                AxRole::Button,
                "Find panel",
                if find_open {
                    EditorActionState {
                        present: true,
                        enabled: false,
                        checked: None,
                    }
                } else {
                    EditorActionState::absent()
                },
            );
            // HBR-QUIET: schedule a repaint only when the present-node set actually changed (IN-041-09).
            if reg.state_changed_since_last_push() {
                ui.ctx().request_repaint();
            }
        }
        // 2) Emit into the live tree + 3) consume this frame's dispatch.
        let (dispatched, to_run) = {
            let reg = wiring.registry.lock().unwrap_or_else(|e| e.into_inner());
            reg.emit_into_tree(ui);
            let dispatched = reg.take_dispatched(ui);
            let to_run: Vec<CodeDispatch> = dispatched
                .iter()
                .filter_map(|aid| {
                    let action_id = Self::strip_code_author_prefix(aid, &handle);
                    CODE_ACTION_CATALOG
                        .iter()
                        .find(|e| e.action_id == action_id)
                        .map(|e| e.dispatch)
                })
                .collect();
            (dispatched, to_run)
        };
        // Run the dispatch targets AFTER dropping the registry lock (a handler may itself touch state).
        for target in to_run {
            self.run_code_dispatch(target);
        }
        dispatched
    }

    /// The live [`EditorActionState`] for one catalog entry, from the real editor state (no mocks).
    fn code_action_state(
        &self,
        entry: &crate::accessibility::editor_action_registry::CodeActionEntry,
        find_open: bool,
        find_state: Option<&FindState>,
        multi_cursor: bool,
    ) -> EditorActionState {
        use crate::accessibility::editor_action_registry::AxRole;
        // Find-step + replace + find-toggle nodes are present ONLY while the find panel is open (their
        // backing widget is not rendered otherwise — AC-041-08).
        let find_scoped = matches!(
            entry.action_id,
            "find-next"
                | "find-prev"
                | "find-toggle-case"
                | "find-toggle-word"
                | "find-toggle-regex"
                | "replace-one"
                | "replace-all"
        );
        let present = if find_scoped {
            find_open
        } else {
            entry.always_present
        };
        if !present {
            return EditorActionState::absent();
        }
        // The language picker AND format are documented gaps (no native language-picker action and no
        // format-document action yet — the keymap has only IndentLine): present but DISABLED so a
        // dispatch is rejected by the MCP channel rather than silently dropped or mis-applied
        // (aliasing format to IndentLine would be a silent wrong action — AC-041-08).
        let enabled = !matches!(
            entry.dispatch,
            CodeDispatch::LanguagePickerUnavailable | CodeDispatch::FormatUnavailable
        ) && (entry.action_id != "multi-cursor-clear" || multi_cursor);
        match entry.role {
            AxRole::Button => EditorActionState {
                present,
                enabled,
                checked: None,
            },
            AxRole::ToggleButton => {
                // The find option toggles reflect the live FindQuery state (RISK-041-03).
                let checked = find_state.map(|f| match entry.action_id {
                    "find-toggle-case" => f.query.case_sensitive,
                    "find-toggle-word" => f.query.whole_word,
                    "find-toggle-regex" => f.query.is_regex,
                    _ => false,
                });
                EditorActionState {
                    present,
                    enabled,
                    checked,
                }
            }
        }
    }

    /// Strip the `editor.code.` prefix (and the optional `.<idx>` instance suffix) from a canonical
    /// author_id, returning the bare `<action>` id the catalog keys on.
    fn strip_code_author_prefix(author_id: &str, handle: &RegistrationHandle) -> String {
        let rest = author_id.strip_prefix("editor.code.").unwrap_or(author_id);
        if let Some(instance_key) = handle.instance_key() {
            let suffix = format!(".{instance_key}");
            return rest.strip_suffix(&suffix).unwrap_or(rest).to_owned();
        }
        // For a non-zero instance the id ends with `.<idx>`; drop it so the catalog lookup matches.
        if handle.instance_index() > 0 {
            let suffix = format!(".{}", handle.instance_index());
            rest.strip_suffix(&suffix).unwrap_or(rest).to_owned()
        } else {
            rest.to_owned()
        }
    }

    /// Run one canonical-action dispatch target against the real panel (the alias-to-real-action step).
    fn run_code_dispatch(&self, target: CodeDispatch) {
        match target {
            CodeDispatch::Action(action) => self.dispatch_action(action),
            CodeDispatch::OpenReplace => self.open_find(true),
            CodeDispatch::ReplaceOne => {
                self.replace_current();
            }
            CodeDispatch::ReplaceAll => {
                self.replace_all();
            }
            CodeDispatch::MultiCursorAdd => self.dispatch_action(CodeEditorAction::AddCursorBelow),
            CodeDispatch::MultiCursorClear => {
                self.dispatch_action(CodeEditorAction::CancelMultiCursor)
            }
            // Flip the one find option, preserving the other two, then re-scan (the real mutator —
            // NOT a re-open of the find panel; mirrors the rich pane's RichDispatch::FindToggle*).
            // A no-op when the find bar is closed (find_state None), matching set_find_toggles.
            CodeDispatch::FindToggleCase => {
                if let Some(q) = self.find_state().map(|f| f.query) {
                    self.set_find_toggles(!q.case_sensitive, q.whole_word, q.is_regex);
                }
            }
            CodeDispatch::FindToggleWord => {
                if let Some(q) = self.find_state().map(|f| f.query) {
                    self.set_find_toggles(q.case_sensitive, !q.whole_word, q.is_regex);
                }
            }
            CodeDispatch::FindToggleRegex => {
                if let Some(q) = self.find_state().map(|f| f.query) {
                    self.set_find_toggles(q.case_sensitive, q.whole_word, !q.is_regex);
                }
            }
            // Disabled nodes — a dispatch should never reach here (the MCP channel rejects a disabled
            // target), but if it does it is a benign no-op + trace, never a silent wrong action.
            CodeDispatch::LanguagePickerUnavailable => {
                tracing::debug!(
                    "editor.code.language-picker-open dispatched but no native language picker exists \
                     (typed gap); no-op"
                );
            }
            CodeDispatch::FormatUnavailable => {
                tracing::debug!(
                    "editor.code.format dispatched but no native format-document action exists \
                     (only IndentLine; typed gap); no-op — never silently indents"
                );
            }
        }
    }

    /// Rebuild the cached AccessKit command-node descriptors iff the keymap version moved since they were
    /// last built (RISK-002 / MC-004 — build the 56-node set ONCE per keymap change, not every frame).
    /// The descriptors carry the fixed/ hashed node id, the `code_editor_cmd_*` author_id, a
    /// chord-annotated label, and the action; the render path emits them as hidden `Role::Button` nodes.
    fn ensure_command_nodes(&self) {
        let version = self.keymap_version.load(Ordering::Relaxed);
        {
            let cache = self
                .command_node_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some((v, _)) = cache.as_ref() {
                if *v == version {
                    return; // up to date for this keymap version.
                }
            }
        }
        let keymap = self
            .keymap
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let descs: Vec<CommandNodeDesc> = CodeEditorAction::all()
            .iter()
            .enumerate()
            .map(|(i, &action)| {
                let author_id = self.command_author_id(action);
                // The bound chord(s) for the action, for the label hint ("Find (Ctrl+F)").
                let chord_hint = keymap
                    .bindings_for_action(action)
                    .first()
                    .map(|b| {
                        let s = KeymapSettings::chord_to_str(&b.chord);
                        match b.second {
                            Some(second) => {
                                format!("{s} {}", KeymapSettings::chord_to_str(&second))
                            }
                            None => s,
                        }
                    })
                    .unwrap_or_default();
                let label = if chord_hint.is_empty() {
                    action.description().to_owned()
                } else {
                    format!("{} ({chord_hint})", action.description())
                };
                // Default panel: a fixed id in the command band; instance panel: a hashed id from the
                // suffixed author_id (RISK-004), the same scheme the other panel nodes use.
                let node_id = if self.instance.is_empty() {
                    // SAFETY: each action index maps to a distinct fixed id in the disjoint command
                    // band (600..656); never reused, cannot self-collide. Same pattern as fold_node_id.
                    unsafe {
                        egui::Id::from_high_entropy_bits(PANEL_COMMAND_NODE_ID_BASE + i as u64)
                    }
                } else {
                    egui::Id::new(&author_id)
                };
                CommandNodeDesc {
                    node_id,
                    author_id,
                    label,
                    action,
                }
            })
            .collect();
        *self
            .command_node_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some((version, descs));
    }

    /// Emit the hidden editor-command AccessKit nodes (AC-005 / HBR-SWARM): one `Role::Button` per
    /// [`CodeEditorAction`], author_id `code_editor_cmd_{name}`, with the `Action::Click`/`Action::Focus`
    /// default actions a swarm agent activates to dispatch the command WITHOUT a keystroke. The nodes
    /// carry NO visual area (they are emitted as zero-size AccessKit nodes, not painted widgets), so they
    /// are invisible to the human operator but present in the tree for agents + the MCP surface. The
    /// descriptors are CACHED per keymap version (RISK-002); only the (cheap) `accesskit_node_builder`
    /// registration runs per frame. Parented to the panel container scope so they are container
    /// descendants like the other editor nodes.
    fn emit_command_nodes(&self, ui: &egui::Ui) {
        self.ensure_command_nodes();
        let cache = self
            .command_node_cache
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let Some((_, descs)) = cache.as_ref() else {
            return;
        };
        for desc in descs {
            let author_id = desc.author_id.clone();
            let label = desc.label.clone();
            ui.ctx().accesskit_node_builder(desc.node_id, move |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(author_id.clone());
                node.set_label(label.clone());
                // The actions a swarm agent dispatches to "press" the hidden command button. Click is the
                // activation; Focus lets an agent move to it first. These are the AccessKit default-action
                // contract for a Button.
                node.add_action(accesskit::Action::Click);
                node.add_action(accesskit::Action::Focus);
            });
        }
    }

    /// Poll the override file for changes and reload the keymap if its mtime moved (implementation note
    /// 6). Stats the file at most once per [`KEYMAP_RELOAD_POLL_SECS`] (a cheap mtime read — NOT the
    /// `notify` crate). A graceful no-op when the file path is unresolvable or the file does not exist.
    /// Called once per frame from `show`.
    fn maybe_reload_keymap(&self) {
        let Some(path) = self.keymap_file_path.as_ref() else {
            return; // no resolvable home dir -> in-memory keymap only.
        };
        // Throttle the stat to once per poll interval.
        {
            let mut state = self
                .keymap_file_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let now = std::time::Instant::now();
            if let Some(last) = state.1 {
                if now.duration_since(last).as_secs() < KEYMAP_RELOAD_POLL_SECS {
                    return;
                }
            }
            state.1 = Some(now);
        }
        // Stat the file's mtime. A missing file is benign (no overrides); only react to a real mtime
        // change so an unchanged file does not reload every poll.
        let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        let changed = {
            let mut state = self
                .keymap_file_state
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let changed = mtime != state.0 && (mtime.is_some() || state.0.is_some());
            state.0 = mtime;
            changed
        };
        if changed {
            match KeymapSettings::load_from_file(path) {
                Ok(settings) => {
                    tracing::info!("keymap.json changed; reloading editor keybindings");
                    self.reload_keymap_from_settings(&settings);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "keymap.json reload failed; keeping current keymap");
                }
            }
        }
    }

    /// MT-010 SINGLE key dispatcher. Reads this frame's key events, resolves each through the active
    /// [`Keymap`] (the one lookup table — replacing the scattered per-feature `egui::Event::Key` arms
    /// MT-003/004/005/006/008 each added), and dispatches the resolved [`CodeEditorAction`] via
    /// [`dispatch_action`](Self::dispatch_action). Handles:
    /// - two-chord prefixes (Ctrl+K then Ctrl+0 -> FoldAll), with the 3-second pending-clear (RISK-001),
    /// - context-sensitive keys (Escape -> Cancel/CloseFind/Dismiss; Tab -> Accept/InsertTab) resolved by
    ///   [`contextual_action`](Self::contextual_action) (step 3).
    ///
    /// This is the ONLY place editor key chords are turned into actions. The live-typing path
    /// (`Event::Text` insert, `Backspace`/`Delete` delete) stays in `process_cursor_input` because it is
    /// character production, not a chord — and the keymap deliberately does not bind printable typing.
    fn process_keymap(&self, ui: &egui::Ui) {
        // MT-048: while a rename is active (the inline input / preview / error is open), the rename surface
        // OWNS the keyboard — the editor body must NOT also process keys, or an Enter that confirms the
        // rename would ALSO insert a newline into the buffer (the focus-precedence bug). The rename's own
        // render path (`render_rename`) reads Enter/Escape; the editor keymap is suppressed entirely this
        // frame so no editor action (InsertNewline / movement / etc.) fires under the open rename input.
        if !matches!(
            *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()),
            RenameState::Idle
        ) {
            return;
        }

        // The find/replace controls are real egui TextEdits rendered after the custom editor body.
        // They therefore own every keyboard event while focused. Preserve only the find lifecycle keys
        // that the product handles itself; all other events must reach the TextEdit without also moving
        // a code caret or editing the code buffer.
        let find_text_surface_owns_keyboard = self.find_text_surface_owns_keyboard();

        // Clear a stale two-chord prefix BEFORE reading events so a timed-out Ctrl+K never wedges
        // single-chord shortcuts (RISK-001 / MC-001 / AC-002 timeout case).
        {
            let mut pending = self.pending_chord.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((_, seen_at)) = *pending {
                if seen_at.elapsed() >= TWO_CHORD_TIMEOUT {
                    *pending = None;
                }
            }
        }

        let events = ui.input(|i| i.events.clone());
        let keymap = self
            .keymap
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        for event in &events {
            let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            else {
                continue;
            };
            if find_text_surface_owns_keyboard {
                if matches!(key, egui::Key::Escape | egui::Key::Enter) {
                    if let ContextOutcome::Dispatch(action) =
                        self.resolve_contextual(*key, modifiers)
                    {
                        self.dispatch_action(action);
                    }
                }
                continue;
            }
            let chord = KeyChord::from_modifiers(*key, modifiers);

            // 1) If a two-chord prefix is pending, this chord must be the SECOND chord.
            let pending_prefix = self
                .pending_chord
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .map(|(c, _)| c);
            if let Some(prefix) = pending_prefix {
                // Clear pending regardless of outcome (a wrong second chord cancels — RISK-001).
                *self.pending_chord.lock().unwrap_or_else(|e| e.into_inner()) = None;
                if let Some(action) = keymap.resolve_second(prefix, chord) {
                    self.dispatch_action(action);
                }
                // Whether or not the second chord matched, this event is consumed by the chord sequence;
                // do NOT also resolve it as a fresh single chord (so Ctrl+0 after Ctrl+K is not also a
                // standalone binding).
                continue;
            }

            // 2) Is this chord the PREFIX of a two-chord binding (e.g. Ctrl+K)? Arm pending + wait.
            if keymap.resolve_prefix(chord) {
                *self.pending_chord.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some((chord, std::time::Instant::now()));
                continue;
            }

            // 3) Context-sensitive override (Escape / Tab / Enter / Arrows) takes precedence over the
            //    raw binding so an open popup / find bar / goto-line palette owns the key (step 3). The
            //    three-state outcome distinguishes "dispatch this action", "consumed — do NOTHING more
            //    this event" (so a goto-line Enter submit does NOT also fall through to InsertNewline),
            //    and "no override — fall through to the plain binding".
            match self.resolve_contextual(*key, modifiers) {
                ContextOutcome::Dispatch(action) => {
                    self.dispatch_action(action);
                    continue;
                }
                ContextOutcome::Consumed => continue,
                ContextOutcome::FallThrough => {}
            }

            // 4) Plain single-chord resolve.
            if let Some(action) = keymap.resolve(chord) {
                self.dispatch_action(action);
            }
        }
    }

    /// Step 3 context-sensitive resolution for the keys whose meaning depends on editor state. Returns a
    /// three-state [`ContextOutcome`]:
    /// - [`ContextOutcome::Dispatch`] — resolve to a state-specific action the dispatcher runs.
    /// - [`ContextOutcome::Consumed`] — the key was handled HERE (e.g. completion select-prev, goto-line
    ///   submit, find next/prev) and must NOT fall through to the plain binding.
    /// - [`ContextOutcome::FallThrough`] — no override; `process_keymap` resolves the plain binding.
    ///
    /// Precedence (matching the prior ad-hoc arms):
    /// - `Escape`: DismissCompletion (popup) > close goto-line (palette) > CloseFind (find) >
    ///   CancelMultiCursor (>1 cursor) > FallThrough (no-op — let the binding's CancelMultiCursor run,
    ///   which for a single caret is a harmless re-collapse).
    /// - `Tab`: AcceptCompletion when the completion popup is open, else FallThrough (InsertTab).
    /// - completion popup open: ArrowUp/Down move the selection (Consumed), Enter accepts (Dispatch).
    /// - goto-line open: Enter submits (Consumed).
    /// - find open: Enter / Shift+Enter step matches (Consumed).
    fn resolve_contextual(&self, key: egui::Key, modifiers: &egui::Modifiers) -> ContextOutcome {
        use egui::Key;
        let completion_open = self.is_completion_open();
        let find_open = self.is_find_open();
        let goto_open = self.is_goto_line_open();
        let symbol_palette_open = self.is_symbol_palette_open();

        // MT-053: while the in-file symbol palette is open it OWNS Up/Down/Enter/Escape (arrow nav,
        // confirm, close) — handled BEFORE the other context keys so the palette behaves like the
        // completion popup / goto-line palette. Up/Down move the selection (Consumed), Enter confirms +
        // jumps (Consumed — no InsertNewline fall-through), Escape closes (Consumed).
        if symbol_palette_open {
            match key {
                Key::Escape => {
                    self.close_symbol_palette();
                    return ContextOutcome::Consumed;
                }
                Key::ArrowDown if !modifiers.ctrl && !modifiers.alt => {
                    self.symbol_palette_select_next();
                    return ContextOutcome::Consumed;
                }
                Key::ArrowUp if !modifiers.ctrl && !modifiers.alt => {
                    self.symbol_palette_select_prev();
                    return ContextOutcome::Consumed;
                }
                Key::Enter => {
                    self.confirm_symbol_palette();
                    return ContextOutcome::Consumed;
                }
                _ => {}
            }
        }

        // Escape is the highest-precedence context key (for the other surfaces).
        if key == Key::Escape {
            return if completion_open {
                ContextOutcome::Dispatch(CodeEditorAction::DismissCompletion)
            } else if goto_open {
                // Close the go-to-line palette directly (consumed; no InsertNewline fall-through).
                self.close_goto_line();
                ContextOutcome::Consumed
            } else if find_open {
                ContextOutcome::Dispatch(CodeEditorAction::CloseFind)
            } else if self
                .cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .len()
                > 1
            {
                ContextOutcome::Dispatch(CodeEditorAction::CancelMultiCursor)
            } else {
                ContextOutcome::FallThrough // nothing open + single caret -> no-op
            };
        }

        // Completion popup owns Up/Down/Enter/Tab while open.
        if completion_open {
            match key {
                Key::ArrowUp if !modifiers.ctrl && !modifiers.alt => {
                    self.completion_select_prev();
                    return ContextOutcome::Consumed;
                }
                Key::ArrowDown if !modifiers.ctrl && !modifiers.alt => {
                    self.completion_select_next();
                    return ContextOutcome::Consumed;
                }
                Key::Enter | Key::Tab => {
                    return ContextOutcome::Dispatch(CodeEditorAction::AcceptCompletion);
                }
                _ => {}
            }
        }

        // Go-to-line palette owns Enter while open (submit). Consumed so Enter does not also insert a
        // newline (the regression this three-state design fixes).
        if goto_open && key == Key::Enter {
            self.submit_goto_line();
            return ContextOutcome::Consumed;
        }

        // Find bar owns Enter / Shift+Enter while open (next / prev match).
        if find_open && key == Key::Enter {
            if modifiers.shift {
                self.prev_match();
            } else {
                self.next_match();
            }
            return ContextOutcome::Consumed;
        }

        ContextOutcome::FallThrough
    }

    /// Dispatch ONE resolved [`CodeEditorAction`] to the appropriate handler. This is the bottom of the
    /// single dispatch path: keymap (or AccessKit command node, or MCP tool) -> action -> handler. Every
    /// branch calls an EXISTING per-feature method (MT-003/004/005/006/008) or a small MT-010 line-edit
    /// helper; no key-event matching happens here.
    pub fn dispatch_action(&self, action: CodeEditorAction) {
        use CodeEditorAction as A;
        match action {
            // ── Caret movement ──
            A::MoveCursorLeft => self.move_cursors(MoveDir::Left, false),
            A::MoveCursorRight => self.move_cursors(MoveDir::Right, false),
            A::MoveCursorUp => self.move_cursors(MoveDir::Up, false),
            A::MoveCursorDown => self.move_cursors(MoveDir::Down, false),
            A::MoveCursorWordLeft => self.move_cursors(MoveDir::WordLeft, false),
            A::MoveCursorWordRight => self.move_cursors(MoveDir::WordRight, false),
            A::MoveCursorLineStart => self.move_cursors(MoveDir::LineStart, false),
            A::MoveCursorLineEnd => self.move_cursors(MoveDir::LineEnd, false),
            A::MoveCursorDocStart => self.move_cursor_doc_edge(true),
            A::MoveCursorDocEnd => self.move_cursor_doc_edge(false),
            // ── Selection (extend) ──
            A::SelectLeft => self.move_cursors(MoveDir::Left, true),
            A::SelectRight => self.move_cursors(MoveDir::Right, true),
            A::SelectUp => self.move_cursors(MoveDir::Up, true),
            A::SelectDown => self.move_cursors(MoveDir::Down, true),
            A::SelectWordLeft => self.move_cursors(MoveDir::WordLeft, true),
            A::SelectWordRight => self.move_cursors(MoveDir::WordRight, true),
            A::SelectLineStart => self.move_cursors(MoveDir::LineStart, true),
            A::SelectLineEnd => self.move_cursors(MoveDir::LineEnd, true),
            A::SelectAll => self.select_all(),
            // ── Deletion ──
            A::DeleteLeft => {
                self.apply_text_edit_undoable("code: delete left", |panel| panel.delete_text());
            }
            A::DeleteRight => {
                self.apply_text_edit_undoable("code: delete right", |panel| panel.delete_forward());
            }
            A::DeleteWordLeft => {
                self.apply_text_edit_undoable("code: delete word left", |panel| {
                    panel.delete_word(true)
                });
            }
            A::DeleteWordRight => {
                self.apply_text_edit_undoable("code: delete word right", |panel| {
                    panel.delete_word(false)
                });
            }
            // MT-051: DeleteLine deletes every affected whole row (incl. trailing newline; the preceding
            // newline too on the last row so no empty trailing line remains) as ONE undo entry.
            A::DeleteLine => {
                self.apply_line_transform("Delete Line", line_ops::delete_line);
            }
            // ── Insertion / line edits ──
            A::InsertNewline => {
                self.apply_text_edit_undoable("code: newline", |panel| panel.insert_text("\n"));
            }
            // MT-051: InsertTab inserts one indent unit (tab or tab_size spaces per the operator setting —
            // MC-006) at every collapsed cursor, OR block-indents when any cursor has a multi-line selection
            // (VS Code parity, AC-005). One undo entry.
            A::InsertTab => {
                self.apply_line_transform("Insert Tab", line_ops::insert_tab);
            }
            // MT-051: Indent/Dedent add/remove one indent unit at each affected line's start (MC-006).
            A::IndentLine => {
                self.apply_line_transform("Indent Line", line_ops::indent_line);
            }
            A::DedentLine => {
                self.apply_line_transform("Dedent Line", line_ops::dedent_line);
            }
            // MT-051: ToggleComment = VS Code all-or-nothing (MC-004) over the affected lines, language-aware
            // (RISK-007; a no-token language is a safe no-op, AC-008). One undo entry.
            A::ToggleComment => {
                self.apply_line_transform("Toggle Comment", line_ops::toggle_comment);
            }
            // MT-051: DuplicateLine copies each affected line below it; the cursor follows to the duplicate.
            A::DuplicateLine => {
                self.apply_line_transform("Duplicate Line", line_ops::duplicate_line);
            }
            // MT-051: MoveLineUp/Down swap the affected line(s) with the neighbor (no-op at the doc edge,
            // MC-005); the cursors travel with their line. One undo entry.
            A::MoveLineUp => {
                self.apply_line_transform("Move Line Up", line_ops::move_line_up);
            }
            A::MoveLineDown => {
                self.apply_line_transform("Move Line Down", line_ops::move_line_down);
            }
            // ── Multi-cursor (existing MT-003 handlers) ──
            A::AddCursorAbove => self.add_cursor_above(),
            A::AddCursorBelow => self.add_cursor_below(),
            A::SelectNextOccurrence => {
                self.select_next_occurrence();
            }
            A::CancelMultiCursor => self.cancel_multi_cursor(),
            // ── Find / replace (existing MT-004 handlers) ──
            A::OpenFind => self.open_find(false),
            A::OpenReplace => self.open_find(true),
            A::FindNext => self.next_match(),
            A::FindPrev => self.prev_match(),
            A::CloseFind => self.close_find(),
            // ── Folding (existing MT-005 handlers + MT-010 all-fold) ──
            A::FoldAtCursor => {
                self.fold_at_cursor();
            }
            A::UnfoldAtCursor => {
                self.unfold_at_cursor();
            }
            A::FoldAll => self.fold_all(true),
            A::UnfoldAll => self.fold_all(false),
            // ── Navigation (existing MT-006/008 handlers) ──
            A::GoToLine => self.toggle_goto_line(),
            A::GoToDefinition => self.request_go_to_definition(),
            A::ShowReferences => self.request_show_references(),
            A::ShowHover => self.request_show_hover(),
            // MT-048: F2 (and the editor body context-menu 'Rename Symbol' entry) begin a rename at the
            // primary caret. `begin_rename` resolves the identifier via tree-sitter and returns None on a
            // non-identifier (so no popup on a keyword/string/whitespace — RISK-006).
            A::RenameSymbol => self.begin_rename_at_cursor(),
            // MT-049: Ctrl+. (and the editor body context-menu 'Quick Fix...' entry) arm a quick-fix
            // request: the per-frame pump fires `textDocument/codeAction` for the current cursor range and
            // OPENS the menu immediately (vs the passive cursor-rest path that only lights the bulb). Armed
            // here so the request runs on the pump (with the live runtime) rather than mid-key-dispatch.
            A::QuickFix => {
                self.quick_fix_request.store(true, Ordering::Relaxed);
            }
            // MT-050: Alt+Shift+F (and the EDIT-menu / editor body context-menu 'Format Document' entry) arm
            // a whole-document format request. The pump fires `textDocument/formatting` off-thread (with the
            // live runtime) and applies the returned TextEdits as one undo step. A no-op + toast when no
            // formatter is available (the disabled keymap path — AC-003); never panics, never blocks.
            A::FormatDocument => self.request_format_document(),
            // MT-050: 'Format Selection' (context-menu / AccessKit node — no default keybinding) arms a
            // `textDocument/rangeFormatting` request for the current selection (empty -> current line).
            A::FormatSelection => self.request_format_selection(),
            // ── Code intelligence (existing MT-008 handlers) ──
            A::TriggerCompletion => {
                self.completion_request
                    .store(COMPLETION_REQUEST_EXPLICIT, Ordering::Relaxed);
                *self
                    .automatic_completion_cursor
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
            }
            A::AcceptCompletion => {
                self.accept_completion();
            }
            A::DismissCompletion => {
                self.close_completion();
                self.close_hover();
                self.close_references();
            }
            // ── History / save / palette ──
            A::Undo => self.undo(),
            A::Redo => self.redo(),
            A::Save => self.request_save(),
            A::OpenCommandPalette => self.open_command_palette(),
            // MT-052 GO-menu navigation. F8 / Shift+F8 traverse the MT-007 diagnostic markers with
            // wraparound (recording the pre-jump location so Back returns); Alt+Left / Alt+Right walk the
            // cross-file jump-history stack. Menu click AND keybinding dispatch THIS same arm (one path
            // through dispatch_action — RISK-007 / MC-007), so the GO menu and the F8/Alt keys never
            // diverge.
            A::GoToNextDiagnostic => self.go_to_next_diagnostic(),
            A::GoToPrevDiagnostic => self.go_to_prev_diagnostic(),
            A::NavigateBack => self.navigate_back(),
            A::NavigateForward => self.navigate_forward(),
            // MT-053: Ctrl+Shift+O (and the GO-menu 'Go to Symbol in File…' item once host-mounted)
            // open the FILE-SCOPED symbol palette. The SAME entry point the menu wiring calls
            // (`open_symbol_palette`) — one path so the menu + the keybind never diverge (AC-005). This
            // is STRICTLY DISTINCT from `OpenCommandPalette` / the MT-030 global quick-switcher.
            A::GoToSymbolInFile => self.open_symbol_palette(),
            // MT-046: 'Copy as note reference' (context-menu / command-node invoked — no default
            // binding). Builds the `[[code:…]]` ref from the live selection/identifier and stages it
            // for the factory render's SHARED-bus clipboard write. The SAME single path the editor
            // body context-menu entry dispatches (one command id, one handler — the MT-010 rule).
            A::CopyAsNoteReference => {
                self.copy_as_note_reference();
            }
        }
    }

    /// Move every cursor in `direction`; when `extend` is true, keep the anchor so the move EXTENDS the
    /// selection (Shift+Arrow), otherwise collapse to a caret (plain Arrow). Reuses the MT-003
    /// [`CursorSet::move_all`] for the collapse case and a per-cursor head move for the extend case.
    fn move_cursors(&self, direction: MoveDir, extend: bool) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let mut set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
        if extend {
            set.extend_all(direction, &buffer);
        } else {
            set.move_all(direction, &buffer);
        }
    }

    /// Move the primary caret to the document start (`to_start`) or end (single caret there — VS Code
    /// Ctrl+Home / Ctrl+End).
    fn move_cursor_doc_edge(&self, to_start: bool) {
        let target = if to_start {
            0
        } else {
            self.with_buffer(|b| b.len_bytes())
        };
        self.set_single_cursor(target);
    }

    /// Select the whole document (one selection spanning the buffer — Ctrl+A).
    fn select_all(&self) {
        let len = self.with_buffer(|b| b.len_bytes());
        self.set_cursors(vec![Cursor::selection(0, len)]);
    }

    /// Forward-delete (Delete key / DeleteRight): delete the selection at each cursor, else the char
    /// AFTER each bare caret. A bare caret at end-of-buffer is a no-op (VS Code Delete-at-EOF does NOT
    /// delete the preceding char). Routed through [`CursorSet::delete_forward_at_all`] so EOF carets
    /// never fall into Backspace semantics — the prior compose-and-delete path ate the preceding char
    /// at EOF (fixed per adversarial review).
    fn delete_forward(&self) -> usize {
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            self.cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .delete_forward_at_all(&mut buffer)
        };
        if applied > 0 {
            self.refresh();
        }
        applied
    }

    /// Delete the word to the left (`to_left`) or right of each cursor (Ctrl+Backspace / Ctrl+Delete):
    /// extend each bare caret over the adjacent word, then delete.
    fn delete_word(&self, to_left: bool) -> usize {
        {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let mut set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            set.select_word_for_bare_carets(to_left, &buffer);
        }
        self.delete_text()
    }

    /// Collapse the cursor set to a single caret at the primary head (Escape with a multi-cursor —
    /// existing MT-003 intent, now named).
    fn cancel_multi_cursor(&self) {
        let head = self
            .cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .primary()
            .head;
        self.set_single_cursor(head);
    }

    /// Fold (`fold`) or unfold ALL foldable regions (Ctrl+K Ctrl+0 / Ctrl+K Ctrl+J). Sets every region's
    /// folded flag, then invalidates the visible map so the next render re-lays the rows.
    fn fold_all(&self, fold: bool) {
        let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
        if set.set_all_folded(fold) {
            drop(set);
            self.fold_state_changed();
        }
    }

    /// Open OR close the go-to-line palette (Ctrl+G toggles; Escape closes via the contextual path).
    fn toggle_goto_line(&self) {
        if self.is_goto_line_open() {
            self.close_goto_line();
        } else {
            self.open_goto_line();
        }
    }

    /// Request a go-to-definition at the primary caret (F12). Looks up the symbol under the caret via the
    /// MT-008 [`CodeNavClient::lookup_symbols`] off-thread, and when the matched symbol carries a
    /// definition span, delivers its 0-based line into [`goto_def_result`](Self::goto_def_result) so the
    /// next frame jumps there via [`navigate_to_line`](Self::navigate_to_line) (the SAME path the hover
    /// "Go to definition" link already uses). A graceful no-op without a bound workspace/runtime or when
    /// the caret is not in a word (HBR-QUIET — never blocks the egui thread, never steals focus).
    fn request_go_to_definition(&self) {
        tracing::debug!("code editor: GoToDefinition (F12) dispatched");
        let Some(runtime) = self.runtime_handle() else {
            return;
        };
        let generation = self.definition_generation.fetch_add(1, Ordering::Relaxed) + 1;
        *self
            .last_definition_target
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .pending_cross_file_jump
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .pending_cross_file_jump_origin
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        let workspace_id = self.workspace_id();
        let word = self.word_at_primary_cursor();
        let lsp = self.lsp_client();
        let document_uri = self.lsp_uri();
        let lsp_available = (lsp.is_configured() || lsp.is_running()) && document_uri.is_some();
        if !lsp_available && (workspace_id.is_empty() || word.is_empty()) {
            return;
        }
        let cursor_byte = self.primary_cursor_offset();
        let request = CodeIntelligenceRequestIdentity {
            generation,
            buffer_version: self.buffer_version.load(Ordering::Relaxed),
            cursor_byte,
            document_uri,
            workspace_id: workspace_id.clone(),
            query: word.clone(),
        };
        let position = self.lsp_position_at(cursor_byte);
        let client = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let cell = Arc::clone(&self.goto_def_result);
        let current_file_path = self.file_path();
        let origin_pane = self
            .host_render_pane_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        runtime.spawn(async move {
            let mut target = if lsp_available {
                lsp.goto_definition_after_sync(
                    request.document_uri.as_deref().unwrap_or_default(),
                    position,
                )
                .await
                .map(navigation_location_from_lsp)
            } else {
                None
            };
            if target.is_none() && !request.workspace_id.is_empty() && !request.query.is_empty() {
                let symbols = client
                    .lookup_symbols(&request.workspace_id, &request.query, 5)
                    .await
                    .unwrap_or_default();
                target = preferred_symbol_for_identifier_in_file(
                    symbols,
                    &request.query,
                    &current_file_path,
                )
                .and_then(|symbol| code_nav_location_from_symbol(&symbol, &current_file_path));
            }
            if let Ok(mut slot) = cell.lock() {
                let delivery = GotoDefinitionDelivery {
                    request,
                    target,
                    origin_pane,
                };
                let replace = slot
                    .as_ref()
                    .map(|current| current.request.generation <= delivery.request.generation)
                    .unwrap_or(true);
                if replace {
                    *slot = Some(delivery);
                }
            }
        });
    }

    /// Request show-references at the primary caret (Shift+F12). LSP is primary; CodeNav resolves the
    /// symbol and its callers/callees as fallback. Both sources deliver normalized, clickable targets
    /// into the same AccessKit overlay. Graceful no-op without either an LSP document or a bound CodeNav
    /// workspace/runtime and word under the caret.
    fn request_show_references(&self) {
        tracing::debug!("code editor: ShowReferences (Shift+F12) dispatched");
        self.close_references();
        let Some(runtime) = self.runtime_handle() else {
            return;
        };
        let generation = self.references_generation.fetch_add(1, Ordering::Relaxed) + 1;
        *self
            .last_references
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        let workspace_id = self.workspace_id();
        let word = self.word_at_primary_cursor();
        let lsp = self.lsp_client();
        let document_uri = self.lsp_uri();
        let lsp_available = (lsp.is_configured() || lsp.is_running()) && document_uri.is_some();
        if !lsp_available && (workspace_id.is_empty() || word.is_empty()) {
            return;
        }
        let cursor_byte = self.primary_cursor_offset();
        let request = CodeIntelligenceRequestIdentity {
            generation,
            buffer_version: self.buffer_version.load(Ordering::Relaxed),
            cursor_byte,
            document_uri,
            workspace_id: workspace_id.clone(),
            query: word.clone(),
        };
        let position = self.lsp_position_at(cursor_byte);
        let client = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let cell = Arc::clone(&self.references_result);
        let current_file_path = self.file_path();
        let references_generation = Arc::clone(&self.references_generation);
        self.last_lsp_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        runtime.spawn(async move {
            let lsp_locations = if lsp_available {
                lsp.references_after_sync(
                    request.document_uri.as_deref().unwrap_or_default(),
                    position,
                )
                .await
            } else {
                Vec::new()
            };
            let payload = if !lsp_locations.is_empty() {
                ReferencesPayload::Lsp(
                    lsp_locations
                        .into_iter()
                        .map(navigation_location_from_lsp)
                        .collect(),
                )
            } else if !request.workspace_id.is_empty() && !request.query.is_empty() {
                let symbols = client
                    .lookup_symbols(&request.workspace_id, &request.query, 5)
                    .await
                    .unwrap_or_default();
                let refs = if let Some(entity_id) = preferred_symbol_for_identifier_in_file(
                    symbols,
                    &request.query,
                    &current_file_path,
                )
                .map(|symbol| symbol.symbol_entity_id)
                .filter(|id| !id.is_empty())
                {
                    client.get_references(&entity_id).await.unwrap_or_default()
                } else {
                    CodeSymbolReferencesResponse::default()
                };
                // The overlay renders at most twenty rows. Never issue hidden N+1 traffic for entries
                // the operator cannot see, and cap concurrent backend resolves at four so a large graph
                // cannot monopolize the runtime. Every task rechecks the shared generation before and
                // after its request; the outer poll aborts the set within 25 ms after dismissal/state
                // invalidation rather than waiting for the HTTP timeout of a stale request.
                let expected_generation = request.generation;
                let limiter = Arc::new(tokio::sync::Semaphore::new(4));
                let mut pending = tokio::task::JoinSet::new();
                for (index, reference) in refs
                    .callers
                    .iter()
                    .chain(refs.callees.iter())
                    .filter(|reference| !reference.symbol_entity_id.is_empty())
                    .take(20)
                    .enumerate()
                {
                    let client = client.clone();
                    let limiter = Arc::clone(&limiter);
                    let generation = Arc::clone(&references_generation);
                    let entity_id = reference.symbol_entity_id.clone();
                    let reference_label = reference.display_name.clone();
                    let current_file_path = current_file_path.clone();
                    pending.spawn(async move {
                        let _permit = limiter.acquire_owned().await.ok()?;
                        if generation.load(Ordering::Relaxed) != expected_generation {
                            return None;
                        }
                        let response = client.get_symbol(&entity_id).await.ok()?;
                        if generation.load(Ordering::Relaxed) != expected_generation {
                            return None;
                        }
                        let target =
                            code_nav_location_from_symbol(&response.symbol, &current_file_path)?;
                        Some((
                            index,
                            CodeReferenceItem {
                                label: if reference_label.is_empty() {
                                    response.symbol.display_name.clone()
                                } else {
                                    reference_label
                                },
                                target,
                            },
                        ))
                    });
                }
                let mut indexed_items = Vec::new();
                while !pending.is_empty() {
                    tokio::select! {
                        joined = pending.join_next() => {
                            if let Some(Ok(Some(item))) = joined {
                                indexed_items.push(item);
                            }
                        }
                        _ = tokio::time::sleep(Duration::from_millis(25)) => {}
                    }
                    if references_generation.load(Ordering::Relaxed) != expected_generation {
                        pending.abort_all();
                        while pending.join_next().await.is_some() {}
                        return;
                    }
                }
                indexed_items.sort_by_key(|(index, _)| *index);
                let items = indexed_items.into_iter().map(|(_, item)| item).collect();
                ReferencesPayload::CodeNav { raw: refs, items }
            } else {
                ReferencesPayload::Lsp(Vec::new())
            };
            if let Ok(mut slot) = cell.lock() {
                let delivery = ReferencesDelivery { request, payload };
                let replace = slot
                    .as_ref()
                    .map(|current| current.request.generation <= delivery.request.generation)
                    .unwrap_or(true);
                if replace {
                    *slot = Some(delivery);
                }
            }
        });
    }

    /// Request a hover at the primary caret (the keymap ShowHover; also fired by dwell in MT-008). Wires
    /// directly to the existing MT-008 [`trigger_hover`](Self::trigger_hover) for the word under the
    /// caret — the SAME working path the hover-dwell pump uses — instead of a placeholder seam. Graceful
    /// no-op without a bound workspace/runtime or word under the caret.
    fn request_show_hover(&self) {
        tracing::debug!("code editor: ShowHover dispatched");
        let Some(runtime) = self.runtime_handle() else {
            return;
        };
        let word = self.word_at_primary_cursor();
        if word.is_empty() {
            return;
        }
        self.trigger_hover(&runtime, &word);
    }

    /// The most recent CodeNav ShowReferences result (callers + callees), or `None` if the current
    /// overlay came from LSP or no CodeNav references request has completed.
    pub fn last_references(&self) -> Option<CodeSymbolReferencesResponse> {
        self.last_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn last_definition_target(&self) -> Option<CodeNavigationLocation> {
        self.last_definition_target
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn last_lsp_references(&self) -> Vec<CodeNavigationLocation> {
        self.last_lsp_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn reference_author_id(&self, index: usize) -> String {
        self.suffixed(&format!("code_editor_reference_{index}"))
    }

    pub fn references_overlay_len(&self) -> usize {
        self.reference_items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .len()
    }

    pub fn close_references(&self) {
        self.references_generation.fetch_add(1, Ordering::Relaxed);
        self.reference_items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        self.last_lsp_references
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        *self
            .last_references
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .references_visible_identity
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    pub fn activate_reference(&self, index: usize) -> bool {
        let target = self
            .reference_items
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(index)
            .map(|item| item.target.clone());
        let Some(target) = target else {
            return false;
        };
        self.apply_code_navigation_target(target);
        self.close_references();
        true
    }

    fn apply_code_navigation_target(&self, target: CodeNavigationLocation) {
        let current_path = self.file_path();
        let target_path = target
            .path
            .as_ref()
            .map(std::path::PathBuf::from)
            .or_else(|| {
                lsp_types::Url::parse(&target.uri)
                    .ok()
                    .and_then(|uri| path_from_lsp_uri(&uri))
            })
            .map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    std::path::Path::new(&current_path)
                        .parent()
                        .filter(|parent| !parent.as_os_str().is_empty())
                        .map(|parent| parent.join(&path))
                        .unwrap_or(path)
                }
            });
        let same_file = self.lsp_uri().as_deref() == Some(target.uri.as_str())
            || target_path.as_ref().is_some_and(|path| {
                let current = std::path::Path::new(&current_path);
                normalized_path_key(current) == normalized_path_key(path)
            });
        if same_file {
            self.record_jump_origin();
            self.navigate_to_line(target.range.start.line as usize);
            *self
                .pending_cross_file_jump
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
            *self
                .pending_cross_file_jump_origin
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = None;
        } else if let Some(path) = target_path {
            self.record_jump_origin();
            self.apply_jump_target(JumpEntry::new(
                path,
                BufferPosition::new(
                    target.range.start.line as usize,
                    target.range.start.character as usize,
                ),
            ));
        }
    }

    fn apply_code_navigation_target_from_origin(
        &self,
        target: CodeNavigationLocation,
        origin_pane: Option<PaneId>,
    ) {
        self.apply_code_navigation_target(target);
        if self.pending_cross_file_jump().is_some() && origin_pane.is_some() {
            *self
                .pending_cross_file_jump_origin
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = origin_pane;
        }
    }

    // ── MT-034 code->notes cross-references (the NoteRefsPanel live wiring) ─────────────────────────────

    /// Show/hide the "Notes referencing this symbol" panel (RISK-001 / MC-001 — hideable like the
    /// outline/minimap). The toggle button in the editor's panel-toggle row flips it; an agent can too.
    pub fn set_show_note_refs(&self, show: bool) {
        let changed = self.show_note_refs.swap(show, Ordering::Relaxed) != show;
        if changed {
            self.reset_note_refs_context();
        }
    }

    /// Whether the NoteRefsPanel is shown.
    pub fn is_note_refs_shown(&self) -> bool {
        self.show_note_refs.load(Ordering::Relaxed)
    }

    /// Flip the NoteRefsPanel visibility (the toggle-button handler).
    fn toggle_note_refs(&self) {
        let now = !self.is_note_refs_shown();
        self.set_show_note_refs(now);
    }

    /// A snapshot of the current NoteRefsPanel load state (for tests / the render path).
    pub fn note_refs_state(&self) -> NoteRefsState {
        self.note_refs_state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// The symbol key the NoteRefsPanel currently tracks (the dwelled symbol), or `None`.
    pub fn note_refs_focused_symbol(&self) -> Option<String> {
        self.note_refs_focused_symbol
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Inject a find-notes search backend (a kittest injects a counted in-memory mock so the live
    /// dwell->search->panel pipeline is driven with NO backend — the MT-014/MT-015 fetcher-trait pattern).
    /// The production default is the verified live search-v2 route ([`FindNotesHttp`]).
    pub fn set_find_notes_backend(&self, backend: Arc<dyn FindNotesSearch>) {
        *self
            .find_notes_backend
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = backend;
        // Replacement changes request ownership even when workspace/caret are unchanged. Clear the
        // active stamp and advance its generation before an old backend can publish late results.
        self.reset_note_refs_context();
    }

    /// Set the cursor-dwell threshold the live `pump_note_refs` uses (default
    /// [`crate::interop::NOTE_REFS_DWELL_MS`]ms). A kittest sets it to ZERO so the dwell->search->panel
    /// pipeline fires on the first settled frame — driving the REAL wired path deterministically without
    /// an 800ms wall-clock wait. Production never calls this (the 800ms default stands).
    pub fn set_note_refs_dwell_threshold(&self, threshold: std::time::Duration) {
        *self
            .note_refs_dwell_threshold
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = threshold;
    }

    /// Invalidate every NoteRefs result tied to the previous workspace/file/caret context. The
    /// generation bump is the async delivery fence: a late A result cannot overwrite a newer B
    /// request, including after a workspace switch.
    fn invalidate_note_refs_request(&self) {
        self.note_refs_generation.fetch_add(1, Ordering::Relaxed);
        *self
            .note_refs_active_request
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        self.note_refs_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clear();
        *self
            .note_refs_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = NoteRefsState::Idle;
        *self
            .note_refs_focused_symbol
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        *self
            .note_refs_dwell
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = SymbolDwellTracker::new();
    }

    fn reset_note_refs_context(&self) {
        self.invalidate_note_refs_request();
        *self
            .note_refs_observed_context
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// MT-034 LIVE code->notes pump: drive the cursor-dwell debounce from the running frame and, on a
    /// dwell crossing (the cursor settled on a NEW symbol for >= [`crate::interop::NOTE_REFS_DWELL_MS`]),
    /// fire the find-notes search OFF-THREAD (RISK-3 / MC-3 — the search fires ONCE per dwell, never per
    /// cursor move / per frame; the debounce suppresses backend spam). The result lands in
    /// [`note_refs_result`](Self::note_refs_result) and the next frame's [`drain_note_refs`](Self::drain_note_refs)
    /// swaps it into `note_refs_state`.
    ///
    /// Resolution: the dwelled WORD is resolved to a `symbol_key` via the SAME MT-008
    /// [`CodeNavClient::lookup_symbols`] path go-to-definition uses, then the precise `symbol_key`
    /// (`path#Symbol`, not a bare word) is the find-notes query — this is the RISK-1 false-positive
    /// mitigation (a qualified key, restricted to rich-doc content types).
    ///
    /// A graceful no-op when: the panel is hidden, no runtime is injected, no workspace is bound, or the
    /// caret is not in a word — so a runtime-less / workspace-less harness renders cleanly while a live
    /// host with a workspace gets real code->notes intelligence. The dwell tracker is driven even while
    /// the panel is hidden is AVOIDED (we skip when hidden so a hidden panel costs nothing).
    fn pump_note_refs(&self) {
        if !self.is_note_refs_shown() {
            return;
        }
        let Some(runtime) = self.runtime_handle() else {
            return; // runtime-less harness: nothing to spawn (graceful).
        };
        let workspace_id = self.workspace_id();
        if workspace_id.is_empty() {
            return;
        }
        let file_path = self.file_path();
        let context = (
            workspace_id.clone(),
            file_path.clone(),
            self.primary_cursor_offset(),
        );
        let context_changed = {
            let mut observed = self
                .note_refs_observed_context
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let changed = observed.as_ref() != Some(&context);
            *observed = Some(context);
            changed
        };
        if context_changed {
            self.invalidate_note_refs_request();
            return;
        }
        // Observe the word under the caret this frame; the dwell tracker fires ONCE per dwell crossing.
        let word = self.word_at_primary_cursor();
        let current = if word.is_empty() {
            None
        } else {
            Some(word.as_str())
        };
        let threshold = *self
            .note_refs_dwell_threshold
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let fired = {
            let mut dwell = self
                .note_refs_dwell
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            dwell.observe_with_threshold(current, std::time::Instant::now(), threshold)
        };
        let Some(dwelled_word) = fired else {
            return; // no dwell crossing this frame -> no search (the debounce suppressed it).
        };

        // A dwell crossed: bind the request to the exact workspace/file/caret generation before the
        // async work starts. Only this stamp may commit its delivery.
        let generation = self.note_refs_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let requested_symbol = format!("{}#{}", file_path.replace('\\', "/"), dwelled_word);
        let stamp = NoteRefsRequestStamp {
            workspace_id: workspace_id.clone(),
            symbol: requested_symbol,
            generation,
        };
        *self
            .note_refs_active_request
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(stamp.clone());
        *self
            .note_refs_state
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = NoteRefsState::Loading;
        *self
            .note_refs_focused_symbol
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(dwelled_word.clone());

        let client = self
            .code_nav_client
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let backend = Arc::clone(
            &self
                .find_notes_backend
                .lock()
                .unwrap_or_else(|e| e.into_inner()),
        );
        let cell = Arc::clone(&self.note_refs_result);
        let ws = workspace_id.clone();
        let request_stamp = stamp.clone();
        let request_file_path = file_path.clone();
        runtime.spawn(async move {
            let lookup = tokio::time::timeout(
                std::time::Duration::from_millis(SYMBOL_KEY_LOOKUP_TIMEOUT_MS),
                client.lookup_symbols(&ws, &dwelled_word, SYMBOL_LOOKUP_LIMIT),
            )
            .await;
            let (projection, candidate_summary) = match lookup {
                Ok(Ok(syms)) => {
                    let candidate_summary = syms
                        .iter()
                        .map(|symbol| format!("{} [{}]", symbol.display_name, symbol.symbol_key))
                        .collect::<Vec<_>>()
                        .join(", ");
                    let projection = preferred_symbol_for_identifier_in_file(
                        syms,
                        &dwelled_word,
                        &request_file_path,
                    )
                    .filter(|symbol| !symbol.symbol_entity_id.trim().is_empty())
                    .filter(|symbol| !symbol.symbol_key.trim().is_empty())
                    .filter(|symbol| {
                        let Some(candidate) = symbol_file_path(&symbol.symbol_key) else {
                            return false;
                        };
                        let mut current = request_file_path.replace('\\', "/");
                        let mut candidate = candidate.replace('\\', "/");
                        #[cfg(windows)]
                        {
                            current.make_ascii_lowercase();
                            candidate.make_ascii_lowercase();
                        }
                        !current.is_empty()
                            && (current == candidate || current.ends_with(&format!("/{candidate}")))
                    });
                    (projection, candidate_summary)
                }
                Ok(Err(error)) => {
                    let delivery = NoteRefsDelivery {
                        stamp: request_stamp,
                        symbol_key: None,
                        state: NoteRefsState::Failed(
                            crate::interop::cross_ref::CrossRefError::Backend(error.to_string()),
                        ),
                    };
                    if let Ok(mut slot) = cell.lock() {
                        slot.push(delivery);
                    }
                    return;
                }
                Err(_) => {
                    let delivery = NoteRefsDelivery {
                        stamp: request_stamp,
                        symbol_key: None,
                        state: NoteRefsState::Failed(
                            crate::interop::cross_ref::CrossRefError::Backend(
                                "symbol lookup timed out".to_owned(),
                            ),
                        ),
                    };
                    if let Ok(mut slot) = cell.lock() {
                        slot.push(delivery);
                    }
                    return;
                }
            };
            let Some(projection) = projection else {
                let delivery = NoteRefsDelivery {
                    stamp: request_stamp,
                    symbol_key: None,
                    state: NoteRefsState::Failed(
                        crate::interop::cross_ref::CrossRefError::NotFound(format!(
                            "no exact symbol projection for '{}' in '{}'; candidates=[{}]",
                            dwelled_word, request_file_path, candidate_summary
                        )),
                    ),
                };
                if let Ok(mut slot) = cell.lock() {
                    slot.push(delivery);
                }
                return;
            };
            let symbol_key = projection.symbol_key.clone();
            let state = match find_code_ref_notes_with(
                backend.as_ref(),
                &projection.symbol_entity_id,
                &symbol_key,
                &ws,
            )
            .await
            {
                Ok(notes) => NoteRefsState::Loaded(notes),
                Err(e) => NoteRefsState::Failed(e),
            };
            if let Ok(mut slot) = cell.lock() {
                slot.push(NoteRefsDelivery {
                    stamp: request_stamp,
                    symbol_key: Some(symbol_key),
                    state,
                });
            }
        });
    }

    /// MT-034: drain a delivered find-notes result into `note_refs_state` (HBR-QUIET — the spawn delivered
    /// it off-thread; here we just swap it in on the UI thread). A no-op when nothing was delivered.
    fn drain_note_refs(&self) {
        let deliveries: Vec<_> = self
            .note_refs_result
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain(..)
            .collect();
        let active = self
            .note_refs_active_request
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let workspace = self.workspace_id();
        if let Some(delivery) = deliveries.into_iter().rev().find(|delivery| {
            active.as_ref() == Some(&delivery.stamp) && workspace == delivery.stamp.workspace_id
        }) {
            if let Some(symbol_key) = delivery.symbol_key {
                *self
                    .note_refs_focused_symbol
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = Some(symbol_key);
            }
            *self
                .note_refs_state
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = delivery.state;
        }
    }

    /// MT-034: render the NoteRefsPanel into `ui` (the right-sidebar surface) and route a clicked note row
    /// through the EXISTING cross-pane Open-Document command on the shared [`InteractionBus`] (reuse, not a
    /// fork). The bus is retrieved from egui app data (the same shared instance every pane uses); the click
    /// is routed with a NON-BLOCKING `try_lock` so a contended frame never deadlocks (RISK-1 / MC-1).
    fn render_note_refs_panel_into(&self, ui: &mut egui::Ui) {
        let theme = if ui.visuals().dark_mode {
            crate::theme::HsTheme::Dark
        } else {
            crate::theme::HsTheme::Light
        };
        let palette = theme.palette();
        let state = self.note_refs_state();
        let focused = self.note_refs_focused_symbol();
        if let Some(doc_id) = render_note_refs_panel(ui, &state, focused.as_deref(), &palette) {
            *self
                .pending_note_ref_open
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some(doc_id);
        }
        let pending = self
            .pending_note_ref_open
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        if let Some(doc_id) = pending {
            let bus = InteractionBus::get_or_init(ui.ctx());
            let delivered = InteractionBus::with_try_lock(&bus, |b| {
                b.register_open_document_command();
                b.open_document(ui.ctx(), doc_id.clone());
            })
            .is_some();
            if delivered {
                *self
                    .pending_note_ref_open
                    .lock()
                    .unwrap_or_else(|e| e.into_inner()) = None;
            } else {
                let response = ui.label(format!("Waiting to open {doc_id}…"));
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    node.set_role(accesskit::Role::Status);
                    node.set_author_id(OPEN_PENDING_AUTHOR_ID.to_owned());
                    node.set_label("Note open pending".to_owned());
                    node.set_value(doc_id.clone());
                });
                ui.ctx().request_repaint();
            }
        }
    }

    /// Undo (Ctrl+Z). Routed to the host command bus: the WP-011 "one unified undo stack across
    /// surfaces" is the shell's responsibility (interconnection_contract / E5), NOT a per-editor undo
    /// buffer. The MT-001 `TextBuffer` deliberately has no undo stack; introducing one here would fork
    /// the unified-undo authority. So the keymap dispatches Undo to the shell, which owns the scope
    /// policy. A no-op + trace when no host bus is wired (headless test).
    fn undo(&self) {
        self.send_to_command_bus(CodeEditorAction::Undo);
    }

    /// WP-KERNEL-012 MT-079 test seam: dispatch Undo through the REAL command channel the keymap uses
    /// (the SAME `send_to_command_bus(CodeEditorAction::Undo)` path Ctrl+Z takes), so the AC-079-3 proof
    /// drives the production dispatch path end-to-end (the shell drain then routes it to the unified-undo
    /// bus) rather than calling the bus directly. Not a tautology: it exercises the mount-installed
    /// command sender.
    pub fn request_undo_for_test(&self) {
        self.undo();
    }

    /// Redo (Ctrl+Y) — routed to the host unified-undo stack, same as [`undo`](Self::undo).
    fn redo(&self) {
        self.send_to_command_bus(CodeEditorAction::Redo);
    }

    /// Save (Ctrl+S). Routes the save intent to the host through the command-palette channel as a Save
    /// action (the document shell owns the actual write — the editor does not write files directly). A
    /// no-op + trace when no host channel is wired.
    fn request_save(&self) {
        self.send_to_command_bus(CodeEditorAction::Save);
    }

    /// WP-KERNEL-012 MT-069: dispatch a Save intent through the EXACT SAME command channel the keymap
    /// Ctrl+S path uses (`send_to_command_bus(CodeEditorAction::Save)`), so a menu-bar / command-palette
    /// "Save" routes to the MT-020 editor save path identically to a keyboard Save — one save substrate,
    /// no shell-local write (MC-004 / RISK-004). The shell drains the channel in `drive_editor_mounts` and
    /// records it as `last_editor_command`; the editor command owns the handshake_core write. Benign no-op
    /// + trace when no host channel is wired (headless).
    pub fn request_save_for_host(&self) {
        self.request_save();
    }

    /// Open the command palette (Ctrl+Shift+P). Routes to the SAME WP-011 command palette via the
    /// injected channel (implementation note — do NOT build a second palette). A no-op + trace when no
    /// host channel is wired.
    fn open_command_palette(&self) {
        self.send_to_command_bus(CodeEditorAction::OpenCommandPalette);
    }

    /// Send an action to the host command bus (the WP-011 command palette / shell command registry) if a
    /// channel is wired. Used for the actions the editor itself cannot complete in-process (Save,
    /// OpenCommandPalette). Benign no-op when no channel is wired (headless test / no host).
    fn send_to_command_bus(&self, action: CodeEditorAction) {
        let tx = self
            .command_palette_tx
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some((tx, document_id)) = tx.as_ref() {
            let _ = tx.send(CodeEditorHostCommand {
                action,
                document_id: document_id.clone(),
                pane_id: self
                    .host_render_pane_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .clone(),
            });
        } else {
            tracing::debug!(
                action = action.name(),
                "code editor command has no host bus; no-op"
            );
        }
    }

    /// Process this frame's egui input for the multi-cursor bindings (MT-003 steps 2-5). Reads pointer
    /// + key events from `ui`'s context:
    /// - Alt+Click -> add a caret at the clicked position; plain Primary click -> single caret.
    /// - Alt+Shift drag -> box/column selection across the dragged line/column range.
    /// - `Event::Text` -> insert the typed text at all cursors (the live typing loop — carried forward
    ///   from MT-003 step 7; the keymap deliberately does not bind printable typing).
    ///
    /// MT-010: the per-feature KEY chords (Ctrl+D, Ctrl+F/H, Ctrl+G, Ctrl+Shift+[/], Ctrl+Alt+Up/Down,
    /// completion-popup keys) are NO LONGER matched here — they go through the single
    /// [`process_keymap`](Self::process_keymap) dispatcher. This method keeps ONLY pointer handling and
    /// the live-typing text/backspace/delete path (character production, not chords).
    fn process_cursor_input(
        &self,
        ui: &egui::Ui,
        _line_height: f32,
        glyph_width: f32,
        total_lines: usize,
    ) {
        // Collect the events we care about in one input read (egui clones cheaply).
        let events = ui.input(|i| i.events.clone());
        let region_rect = self
            .row_geometry
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .map(|g| {
                // The painted region: from the top-left origin down over the painted rows, full width.
                let height = (total_lines.saturating_sub(g.first_line)) as f32 * g.line_height;
                egui::Rect::from_min_size(
                    egui::pos2(g.left, g.top),
                    egui::vec2(ui.clip_rect().width().max(1.0), height.max(g.line_height)),
                )
            });

        for event in &events {
            match event {
                // POINTER: Alt+Click adds a cursor; plain Primary click sets a single cursor. Box drag
                // (Alt+Shift) is handled by drag start/end below via the same press/release events.
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    pos,
                    modifiers,
                } => {
                    // Only react when the press is inside the editor row region (avoid hijacking shell
                    // clicks).
                    if region_rect.map(|r| r.contains(*pos)).unwrap_or(false) {
                        if modifiers.alt && modifiers.shift {
                            // Begin a box/column selection drag: remember the (line, col) start.
                            if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines)
                            {
                                let (line, col) = self.with_buffer(|b| byte_to_line_col(byte, b));
                                *self
                                    .box_drag_start
                                    .lock()
                                    .unwrap_or_else(|e| e.into_inner()) = Some((line, col));
                            }
                        } else if modifiers.alt {
                            if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines)
                            {
                                self.add_cursor_at(byte);
                            }
                        } else {
                            // Plain click: single caret + clear any box drag.
                            *self
                                .box_drag_start
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) = None;
                            if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines)
                            {
                                self.set_single_cursor(byte);
                            }
                        }
                    }
                }
                // POINTER RELEASE: finish an Alt+Shift box-selection drag.
                egui::Event::PointerButton {
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    pos,
                    modifiers,
                } => {
                    let start = self
                        .box_drag_start
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .take();
                    if let (Some((sl, sc)), true) = (start, modifiers.alt && modifiers.shift) {
                        if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines) {
                            let (el, ec) = self.with_buffer(|b| byte_to_line_col(byte, b));
                            self.set_box_selection(sl, sc, el, ec);
                        }
                    }
                }
                // LIVE TYPING (carried forward from MT-003 step 7): a typed character is inserted at
                // EVERY cursor (the core editor typing loop — Event::Text -> CursorSet::insert_at_all
                // via `insert_text`, which bumps buffer_version for the MT-002 highlight-cache
                // invalidation). The keymap deliberately does NOT bind printable typing, so this is the
                // ONE place text production happens. It also marks the MT-008 completion-debounce clock
                // and, on a completion TRIGGER character (`.`/`:`/`_`), arms a completion request for
                // this frame's pump. The completion popup is non-focus-stealing (RISK-005), so the
                // character still lands. egui never emits an Event::Text for a chord (Ctrl+C etc.), so a
                // shortcut does not also type a character.
                egui::Event::Text(text) if !text.is_empty() => {
                    // MT-048: while the rename input is open (Editing phase) the FOCUSED input owns typed
                    // text — do NOT also insert it into the editor buffer (the focus-precedence rule). A
                    // preview/error phase has no text target either; skip in any non-Idle rename phase.
                    if !matches!(
                        *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()),
                        RenameState::Idle
                    ) || self.find_text_surface_owns_keyboard()
                    {
                        continue;
                    }
                    // Skip while a completion popup is open AND the text would be consumed by an accept —
                    // but the popup is non-focus-stealing, so normal typing still flows; only the explicit
                    // Tab/Enter accept (handled in process_keymap) consumes. Insert the text at all
                    // cursors.
                    if self
                        .apply_text_edit_undoable("code: typing", |panel| panel.insert_text(text))
                        > 0
                    {
                        self.mark_edit_now();
                    }
                    if text.chars().any(|c| matches!(c, '.' | ':' | '_')) {
                        self.completion_request
                            .fetch_max(COMPLETION_REQUEST_AUTOMATIC, Ordering::Relaxed);
                        if self.completion_request.load(Ordering::Relaxed)
                            == COMPLETION_REQUEST_AUTOMATIC
                        {
                            *self
                                .automatic_completion_cursor
                                .lock()
                                .unwrap_or_else(|e| e.into_inner()) =
                                Some(self.primary_cursor_offset());
                        }
                    }
                    // MT-047 signature help: an open-paren OPENS the popup at a new call site; a comma
                    // UPDATES the active parameter of the open popup (the pump keys it by the call's
                    // open-paren). A close-paren `)` DISMISSES the popup (the call's argument list ended).
                    // The popup is non-focus-stealing, so the character still lands (RISK-003).
                    if text.chars().any(|c| matches!(c, '(' | ',')) {
                        self.signature_help_request.store(true, Ordering::Relaxed);
                    }
                    if text.contains(')') {
                        // Only dismiss when the cursor has actually left the call (paren balance);
                        // `trigger_signature_help` re-evaluates on the next pump and re-opens if the
                        // cursor is still inside an outer call, so a nested `)` does not wrongly close.
                        self.signature_help_request.store(true, Ordering::Relaxed);
                    }
                }
                // WP-KERNEL-012 MT-076 (E13 IME / AC5): IME composition. `Enabled`/`Preedit` set the
                // OVERLAY preedit text (NO buffer mutation — RISK-1 / MC-1); only `Commit` INSERTS the
                // composed text at all cursors via the proven char-correct `insert_text` path, so CJK
                // composes + commits into a code buffer exactly like the rich editor. An empty commit /
                // `Disabled` clears the overlay with no insert (cancel path). Skipped while a rename input
                // owns typed input (the same focus-precedence rule as `Event::Text`). egui's IME events are
                // distinct from `Event::Text`, so committed CJK does not double-insert.
                egui::Event::Ime(ime) => {
                    if !matches!(
                        *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()),
                        RenameState::Idle
                    ) || self.find_text_surface_owns_keyboard()
                    {
                        continue;
                    }
                    let before = self.buffer();
                    if self.handle_ime_event(ime) {
                        // A non-empty commit mutated the buffer — mark the edit (debounce clock) so the
                        // MT-008 completion + MT-035 dirty/draft tracking treat it like typed text.
                        self.record_text_edit_undo(before, self.buffer(), "code: ime commit");
                        self.mark_edit_now();
                    }
                }
                _ => {}
            }
        }

        // MT-047 signature-help keys, detected via input STATE queries (NOT an `egui::Event::Key`
        // match arm — MT-010's single-dispatch invariant keeps `egui::Event::Key` to the one
        // `process_keymap` site; `keymap.rs`'s `CodeEditorAction` enum is out of MT-047 scope). The
        // popup is non-focus-stealing, so reading these here does not consume the chord from the editor:
        // - Ctrl+Shift+Space: arm a manual signature-help request (the manual VS Code shortcut).
        // - Escape: dismiss the popup when it is open.
        // - Up/Down: cycle overloads while the popup is open.
        let (sig_manual, sig_escape, sig_up, sig_down) = ui.input(|i| {
            let m = i.modifiers;
            (
                m.ctrl && m.shift && i.key_pressed(egui::Key::Space),
                i.key_pressed(egui::Key::Escape),
                i.key_pressed(egui::Key::ArrowUp),
                i.key_pressed(egui::Key::ArrowDown),
            )
        });
        if sig_manual {
            self.signature_help_request.store(true, Ordering::Relaxed);
        }
        if self.is_signature_help_open() {
            if sig_escape {
                self.close_signature_help();
                // AC-002 double-fire fix: Escape is read here via an input-STATE query (peeked, not
                // consumed), so without this `process_keymap` (below) would ALSO resolve Escape the SAME
                // frame and run CancelMultiCursor — collapsing a multi-cursor / selection while merely
                // dismissing the popup. Consume it so dismissing the popup does NOT also fire an editor
                // action. Skip the consume when the completion popup is open: it OWNS Escape
                // (`DismissCompletion`, which `resolve_contextual` already returns as `Consumed` with no
                // CancelMultiCursor fall-through), so leaving the event lets completion dismiss too with
                // no double action.
                if !self.is_completion_open() {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                }
            } else if sig_down {
                self.signature_help_next();
                // Parity hardening (same peeked-not-consumed class as Escape): when there is MORE THAN ONE
                // overload the popup OWNS Up/Down (cycle the active signature) like the completion popup
                // does — consume so the key does not ALSO fall through to `process_keymap`'s line movement.
                // With a single overload the cycle is a no-op, so the key is left to move the caret (which
                // then dismisses the popup via the per-frame guard).
                if self.signature_help_overload_count() > 1 {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
                }
            } else if sig_up {
                self.signature_help_prev();
                if self.signature_help_overload_count() > 1 {
                    ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
                }
            }
        }

        // MT-010: ALL key-chord handling is consolidated into the single keymap dispatcher. The
        // per-feature `egui::Event::Key` match arms MT-003/004/005/006/008 each added here are GONE —
        // `process_keymap` resolves every chord through the one `Keymap` table and dispatches the
        // resolved `CodeEditorAction` (including the live-typing Backspace/Delete via DeleteLeft/
        // DeleteRight). Run AFTER the pointer + text handling above so a click-then-key in the same
        // frame sees the updated caret.
        // An open egui popup owns navigation/confirmation keys. In particular, the editor-body context
        // menu consumes ArrowUp/ArrowDown/Enter after the body has already processed input in this render
        // order; dispatching the editor keymap here as well would collapse/move the live code selection
        // before a menu command (such as Copy as note reference) reads it.
        if !egui::Popup::is_any_open(ui.ctx()) {
            self.process_keymap(ui);
        }

        // WP-KERNEL-012 MT-041 (E7): sync + emit the consolidated `editor.code.<action>` AccessKit nodes
        // and consume any swarm `Action::Click` dispatched at them THIS frame, so a swarm agent's
        // dispatch reaches the editor before the next frame (RISK-041-04). A no-op when no registry is
        // installed (a bare panel render). Run last so it sees the post-input editor state.
        let _dispatched = self.sync_editor_actions(ui);

        // WP-KERNEL-012 MT-080 (AC-080-6 / MT-043): drain any swarm `Action::SetValue` /
        // `Action::ReplaceSelectedText` dispatched at the code-editor-text node THIS frame and apply it to
        // the buffer, so a swarm agent can AUTHOR code by id within the frame (the same in-frame consume
        // discipline `sync_editor_actions` uses for `Click`). A no-op when no such request was dispatched.
        self.consume_swarm_text_actions(ui);
        // MT-108: consume a SetValue dispatched at the stable find-bar node and route it through the same
        // setter as the live TextEdit, preserving incremental re-search and normal UI state transitions.
        self.consume_swarm_find_actions(ui);
    }

    /// WP-KERNEL-012 MT-080 (AC-080-6 / MT-043 swarm-authoring): drain this frame's swarm text-edit
    /// requests targeted at the `editor.code.text` node and apply each to the buffer. Two actions:
    /// - [`accesskit::Action::SetValue`] with an [`accesskit::ActionData::Value`] payload replaces the
    ///   WHOLE buffer ([`set_text`](Self::set_text)) — the swarm "author the whole file" path.
    /// - [`accesskit::Action::ReplaceSelectedText`] with a `Value` payload inserts the text at the
    ///   selection/carets ([`insert_text`](Self::insert_text), which replaces the active selection) — the
    ///   swarm "edit the selection" path. The host's wikilink-by-id insertion (a `[[id]]` reference) rides
    ///   this same path: the agent dispatches `ReplaceSelectedText` with the wikilink token as the value.
    ///
    /// Reuses egui's own `input.accesskit_action_requests(node_id, action)` consumer (the same hook the
    /// MT-041 registry uses), so a swarm agent's `egui::Event::AccessKitActionRequest` drives the node
    /// exactly like a real edit. The byte length the apply returns is recorded so a test can observe the
    /// dispatch reached the buffer; an empty/absent `Value` is a benign no-op (never a panic).
    pub fn consume_swarm_text_actions(&self, ui: &egui::Ui) {
        // Read action requests at the LIVE text-node id the render path recorded this frame (the node is
        // emitted on `ui.unique_id()` inside the text scope, NOT on `text_id()`). Before the first render
        // there is no live id, so there is nothing to consume.
        let Some(text_id) = *self
            .live_text_node_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        else {
            return;
        };
        // Collect the (action, value) pairs first so the input lock is released before mutating the buffer
        // (set_text/insert_text take their own locks). SetValue is applied before ReplaceSelectedText so a
        // whole-buffer set followed by a selection-insert in the same frame composes deterministically.
        let mut focus = false;
        let mut set_value: Option<String> = None;
        let mut replace_values: Vec<String> = Vec::new();
        ui.input(|input| {
            let focus_requested = input
                .accesskit_action_requests(text_id, accesskit::Action::Focus)
                .next()
                .is_some();
            let click_requested = input
                .accesskit_action_requests(text_id, accesskit::Action::Click)
                .next()
                .is_some();
            focus = focus_requested || click_requested;
            for request in input.accesskit_action_requests(text_id, accesskit::Action::SetValue) {
                if let Some(accesskit::ActionData::Value(v)) = &request.data {
                    set_value = Some(v.to_string());
                }
            }
            for request in
                input.accesskit_action_requests(text_id, accesskit::Action::ReplaceSelectedText)
            {
                if let Some(accesskit::ActionData::Value(v)) = &request.data {
                    replace_values.push(v.to_string());
                }
            }
        });
        if focus {
            ui.ctx().memory_mut(|memory| memory.request_focus(text_id));
            ui.ctx().request_repaint();
        }
        if let Some(value) = set_value {
            let before = self.buffer();
            self.set_text(&value);
            self.record_text_edit_undo(before, self.buffer(), "code: swarm set value");
            ui.ctx().request_repaint();
        }
        for value in replace_values {
            if !value.is_empty() {
                self.apply_text_edit_undoable("code: swarm replace selected text", |panel| {
                    panel.insert_text(&value)
                });
                ui.ctx().request_repaint();
            }
        }
    }

    /// MT-108: drain Argus `SetValue` requests targeted at the stable find-bar node and update the real
    /// query state. The stable node is intentionally separate from egui's generated TextEdit node, so
    /// this bridge consumes the request at the exact discoverable author id and delegates to
    /// [`Self::set_find_query`] for the canonical re-search path. A missing/closed bar is a no-op.
    fn consume_swarm_find_actions(&self, ui: &egui::Ui) {
        let Some(find_node_id) = *self
            .live_find_node_id
            .lock()
            .unwrap_or_else(|e| e.into_inner())
        else {
            return;
        };
        let mut value = None;
        ui.input(|input| {
            for request in
                input.accesskit_action_requests(find_node_id, accesskit::Action::SetValue)
            {
                if let Some(accesskit::ActionData::Value(next)) = &request.data {
                    value = Some(next.to_string());
                }
            }
        });
        if let Some(value) = value {
            self.set_find_query(value);
            ui.ctx().request_repaint();
        }
    }
}

/// A [`PaneFactory`] that mounts a [`CodeEditorPanel`] as a named work-surface pane (MT-001 step 5).
/// Registered for [`PaneType::CodeSymbol`] (the closest existing WP-011 pane variant for a code
/// surface) so the editor appears in the WP-011 docking split layout through the EXISTING pane
/// registry + split layout — no new shell infrastructure is forked.
struct PendingCodeEditFlightEvent {
    last_change: Instant,
    line_delta: i64,
    pane_id: String,
    workspace_id: String,
    file_path: String,
}

pub struct CodeEditorPaneFactory {
    panel: Arc<CodeEditorPanel>,
    /// MT-031: set once after the code surface registers its melt-together command set into the shared
    /// bus, so re-registration is idempotent across frames (interior-mutable: the registry borrows
    /// `&dyn PaneFactory` at render time, so `render` has no `&mut self`).
    bus_registered: std::sync::atomic::AtomicBool,
    /// MT-036: one 2-second trailing-edge Flight Recorder batch per mounted code pane. Render only
    /// updates/polls this tiny state; transport dispatch stays on the existing non-blocking emitter.
    pending_code_edit_event: Mutex<Option<PendingCodeEditFlightEvent>>,
    /// Identity changes (workspace/pane/file) close the prior batch without relabelling it. Closed
    /// batches retain their own trailing-edge deadline and drain in causal order before the active batch.
    queued_code_edit_events: Mutex<VecDeque<PendingCodeEditFlightEvent>>,
}

impl CodeEditorPaneFactory {
    const CODE_EDIT_DEBOUNCE: Duration = Duration::from_secs(2);

    /// Build a factory wrapping `panel`. `Arc` so the same panel renders across frames without the
    /// factory owning a `&mut` (the registry borrows `&dyn PaneFactory` at render time).
    pub fn new(panel: CodeEditorPanel) -> Self {
        Self {
            panel: Arc::new(panel),
            bus_registered: std::sync::atomic::AtomicBool::new(false),
            pending_code_edit_event: Mutex::new(None),
            queued_code_edit_events: Mutex::new(VecDeque::new()),
        }
    }

    /// WP-KERNEL-012 MT-079: build a factory over an EXISTING `Arc<CodeEditorPanel>` so the
    /// session-threading host-mount wrapper (`editor_pane_factories::CodeEditorPaneMount`) and this
    /// inner factory render the SAME panel state. `new` wraps a fresh panel in its own Arc, which would
    /// give the mount and the inner render two different panels; this constructor shares one Arc.
    pub fn from_arc(panel: Arc<CodeEditorPanel>) -> Self {
        Self {
            panel,
            bus_registered: std::sync::atomic::AtomicBool::new(false),
            pending_code_edit_event: Mutex::new(None),
            queued_code_edit_events: Mutex::new(VecDeque::new()),
        }
    }

    /// The Arc-shared panel this factory renders (so a test/host can drive the SAME panel state the
    /// mounted pane shows — MT-031 cross-pane proof needs the real panel behind the factory).
    pub fn panel(&self) -> Arc<CodeEditorPanel> {
        Arc::clone(&self.panel)
    }

    fn stage_code_edit_event_at(
        &self,
        pane_id: &str,
        workspace_id: &str,
        file_path: &str,
        line_delta: i64,
        now: Instant,
    ) {
        let new_batch = || PendingCodeEditFlightEvent {
            last_change: now,
            line_delta,
            pane_id: pane_id.to_owned(),
            workspace_id: workspace_id.to_owned(),
            file_path: file_path.to_owned(),
        };
        let displaced = {
            let mut pending = self
                .pending_code_edit_event
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match pending.as_mut() {
                Some(batch)
                    if batch.pane_id == pane_id
                        && batch.workspace_id == workspace_id
                        && batch.file_path == file_path =>
                {
                    batch.last_change = now;
                    batch.line_delta = batch.line_delta.saturating_add(line_delta);
                    None
                }
                Some(_) => pending.replace(new_batch()),
                None => {
                    *pending = Some(new_batch());
                    None
                }
            }
        };
        if let Some(displaced) = displaced {
            self.queued_code_edit_events
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .push_back(displaced);
        }
    }

    /// Take one due event at an injected monotonic time. Keeping the clock at this boundary makes the
    /// trailing-edge contract deterministic in tests without sleeps or scheduler timing.
    fn take_due_code_edit_event_at(
        &self,
        now: Instant,
    ) -> Option<crate::event_emitter::NativeEditorEvent> {
        let queued_batch = {
            let mut queued = self
                .queued_code_edit_events
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match queued.front() {
                Some(batch)
                    if now
                        .checked_duration_since(batch.last_change)
                        .unwrap_or_default()
                        >= Self::CODE_EDIT_DEBOUNCE =>
                {
                    queued.pop_front()
                }
                Some(_) => return None,
                None => None,
            }
        };
        let batch = queued_batch.or_else(|| {
            let mut pending = self
                .pending_code_edit_event
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            match pending.as_ref() {
                Some(batch)
                    if now
                        .checked_duration_since(batch.last_change)
                        .unwrap_or_default()
                        >= Self::CODE_EDIT_DEBOUNCE =>
                {
                    pending.take()
                }
                Some(_) => None,
                None => None,
            }
        });
        batch.map(|batch| {
            let file_path = if batch.file_path.trim().is_empty() {
                batch.pane_id.clone()
            } else {
                batch.file_path
            };
            crate::event_emitter::NativeEditorEvent::code_edit(
                file_path,
                batch.line_delta,
                batch.pane_id.clone(),
                crate::event_emitter::native_editor_actor_id(&batch.pane_id),
                batch.workspace_id,
            )
        })
    }

    fn flush_code_edit_event_at(&self, ctx: &egui::Context, now: Instant) {
        if let Some(event) = self.take_due_code_edit_event_at(now) {
            crate::event_emitter::dispatch_event_from_frame(ctx, event);
            ctx.request_repaint();
            return;
        }
        let repaint_after = self
            .queued_code_edit_events
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .front()
            .map(|batch| batch.last_change)
            .or_else(|| {
                self.pending_code_edit_event
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .as_ref()
                    .map(|batch| batch.last_change)
            })
            .map(|batch| {
                Self::CODE_EDIT_DEBOUNCE
                    .saturating_sub(now.checked_duration_since(batch).unwrap_or_default())
            });
        if let Some(delay) = repaint_after {
            ctx.request_repaint_after(delay);
        }
    }
}

impl PaneFactory for CodeEditorPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::CodeSymbol
    }

    fn render(&self, ui: &mut egui::Ui, ctx: &PaneRenderContext) {
        // MT-031 (E5 melt-together) LIVE WIRING: a MOUNTED code pane retrieves the ONE shared bus and
        // publishes its selection + registers its command set every frame — this is the real per-frame
        // bus consumer the contract requires (not test-only dead code). The bus lives in egui app data
        // keyed by INTERACTION_BUS_KEY, so every mounted pane sees the same instance.
        let bus = crate::interop::interaction_bus::InteractionBus::get_or_init(ui.ctx());
        let pane_id: PaneId = Arc::from(ctx.record.pane_id.as_ref());
        // The code pane owns shared focus only while the pane's egui scope is actually focused. A stale
        // code selection must not claim focus or consume clipboard shortcuts while another editor/input is
        // active; explicit menu/palette commands materialize selections through the host dispatch path.
        let has_focus = ui.memory(|m| m.focused().map(|f| f == ctx.egui_id).unwrap_or(false))
            || self.panel.live_text_has_focus(ui.ctx());
        // MT-047 (AC-002): mirror the real pane focus into the panel BEFORE `show()` so the per-frame
        // signature-help dismissal guard closes the popup when the code editor loses focus (scope step 8).
        self.panel.set_code_surface_focus(has_focus);
        let mut registered = self.bus_registered.load(Ordering::Relaxed);
        super::interop_adapter::drive_bus_in_render(
            &bus,
            &self.panel,
            pane_id.clone(),
            has_focus,
            &mut registered,
        );
        self.bus_registered.store(registered, Ordering::Relaxed);
        if super::interop_adapter::drive_clipboard_shortcuts_in_render(
            ui,
            &bus,
            &self.panel,
            pane_id,
            has_focus,
        ) {
            ui.ctx().request_repaint();
        }

        self.panel.show(ui);

        // MT-035: live typing / IME / delete / newline record their undo snapshots through the SAME bus
        // boundary as format and line transforms. Do not take the staged snapshot until the bus lock is
        // acquired; if the bus is contended this frame, request a repaint and preserve the pending entry.
        if self.panel.has_pending_text_edit_undo() {
            let pane_id_text: PaneId = Arc::from(ctx.record.pane_id.as_ref());
            let drained =
                crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
                    let Some(pending) = self.panel.take_pending_text_edit_undo() else {
                        return false;
                    };
                    b.set_focus_owner(pane_id_text.clone());
                    super::interop_adapter::push_or_coalesce_code_edit_undo(
                        b,
                        pane_id_text.clone(),
                        &self.panel,
                        pending.before,
                        pending.after,
                        pending.description,
                        pending.replace_tail,
                    );
                    true
                })
                .unwrap_or(false);
            if !drained {
                ui.ctx().request_repaint();
            }
        }

        // MT-050 (AC-001): record the SINGLE undo entry for a just-applied format at the SAME bus boundary
        // every code edit's undo is recorded (the wrap-not-fork discipline). The panel queued the
        // (before, after) snapshot during its format drain; push it as ONE `UndoAction` so a single Ctrl+Z
        // reverts the entire format. `with_try_lock` so it never blocks the egui frame thread (RISK-1).
        if let Some((before, after)) = self.panel.take_pending_format_undo() {
            let pane_id2: PaneId = Arc::from(ctx.record.pane_id.as_ref());
            crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
                super::interop_adapter::push_code_edit_undo(
                    b,
                    pane_id2.clone(),
                    &self.panel,
                    TextBuffer::new(&before),
                    TextBuffer::new(&after),
                    "Format Document",
                );
            });
        }

        // MT-051 (AC-007): record the SINGLE undo entry for a just-applied line transform (ToggleComment /
        // DuplicateLine / MoveLine / DeleteLine / Indent / Dedent / InsertTab) at the SAME bus boundary
        // every code edit's undo is recorded at. The panel queued the (before, after) snapshot during the
        // transform; push it as ONE `UndoAction` so a single Ctrl+Z reverts the whole transform across all
        // affected lines + cursors. `with_try_lock` so it never blocks the egui frame thread (RISK-1).
        if let Some((description, before, after)) = self.panel.take_pending_line_op_undo() {
            let pane_id3: PaneId = Arc::from(ctx.record.pane_id.as_ref());
            crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
                super::interop_adapter::push_code_edit_undo(
                    b,
                    pane_id3.clone(),
                    &self.panel,
                    TextBuffer::new(&before),
                    TextBuffer::new(&after),
                    description,
                );
            });
        }

        // MT-036/MT-069: every successful buffer mutation records exactly one lightweight receipt on
        // the panel. Drain all receipts accumulated since the prior frame into this pane's existing
        // two-second trailing-edge batch, then poll the due event. Failed/no-op/cancel paths never create
        // a receipt, so they cannot emit. One monotonic sample drives both stage and flush deterministically.
        let code_edit_now = Instant::now();
        for receipt in self.panel.take_pending_code_edit_receipts() {
            let pane_id = receipt
                .pane_id
                .as_deref()
                .unwrap_or(ctx.record.pane_id.as_ref());
            self.stage_code_edit_event_at(
                pane_id,
                &receipt.workspace_id,
                &receipt.file_path,
                receipt.line_delta,
                code_edit_now,
            );
        }
        self.flush_code_edit_event_at(ui.ctx(), code_edit_now);

        // MT-046: a 'Copy as note reference' dispatch staged its `[[code:…]]` ref this frame — write it
        // to the SHARED bus clipboard through the SAME mockable-sink path Ctrl+C uses (the production
        // sink is the egui `copy_text` surface; headless kittest runs never touch the OS clipboard).
        // Guarded by a cheap peek-free take inside the try-lock so an idle frame costs one lock try.
        crate::interop::interaction_bus::InteractionBus::with_try_lock(&bus, |b| {
            let sink = crate::rich_editor::properties::metadata_client::EguiClipboard::new(
                ui.ctx().clone(),
            );
            let _ = super::interop_adapter::copy_note_reference_to_bus(b, &self.panel, &sink);
        });
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn large_initial_highlight_source() -> String {
        (0..5_100)
            .map(|line| format!("fn item_{line}() -> usize {{ {line} }}\n"))
            .collect()
    }

    fn await_initial_highlight_terminal(panel: &CodeEditorPanel) -> InitialHighlightStatus {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let status = panel.initial_highlight_status();
            if status != InitialHighlightStatus::Pending || Instant::now() >= deadline {
                return status;
            }
            std::thread::yield_now();
        }
    }

    #[test]
    fn initial_highlight_worker_faults_retry_off_ui_and_preserve_initial_window() {
        for (fault, expected_failure) in [
            (
                InitialHighlightTestFault::SpawnUnavailable,
                InitialHighlightFailure::WorkerUnavailable,
            ),
            (
                InitialHighlightTestFault::Disconnect,
                InitialHighlightFailure::WorkerUnavailable,
            ),
            (
                InitialHighlightTestFault::WorkerPanicked,
                InitialHighlightFailure::WorkerPanicked,
            ),
            (
                InitialHighlightTestFault::EmptyProjection,
                InitialHighlightFailure::EmptyProjection,
            ),
            (
                InitialHighlightTestFault::StaleGeneration,
                InitialHighlightFailure::StaleDelivery,
            ),
        ] {
            let panel = CodeEditorPanel::new(&large_initial_highlight_source(), "rs");
            let initial_spans = panel.initial_span_count();
            assert!(initial_spans > 0, "fixture must emit a foreground window");
            panel.inject_initial_highlight_fault(fault);
            assert_eq!(
                await_initial_highlight_terminal(&panel),
                InitialHighlightStatus::Complete,
                "recoverable {fault:?} must complete through the bounded worker retry"
            );
            assert_eq!(
                panel.initial_highlight_failure(),
                Some(expected_failure),
                "the recovered worker failure remains observable as a typed diagnostic"
            );
            assert!(panel.span_count() >= initial_spans);
            assert!(
                panel
                    .initial_highlight_source
                    .lock()
                    .unwrap_or_else(|error| error.into_inner())
                    .is_none(),
                "terminal delivery must release the retained source"
            );
        }
    }

    #[test]
    fn initial_highlight_queue_full_is_non_blocking_and_retries_submission() {
        let panel = CodeEditorPanel::new(&large_initial_highlight_source(), "rs");
        panel.inject_initial_highlight_fault(InitialHighlightTestFault::QueueFull);
        assert_eq!(
            panel.initial_highlight_status(),
            InitialHighlightStatus::Pending
        );
        assert_eq!(
            panel.initial_highlight_failure(),
            Some(InitialHighlightFailure::QueueSaturated)
        );
        assert_eq!(
            await_initial_highlight_terminal(&panel),
            InitialHighlightStatus::Complete
        );
    }

    #[test]
    fn initial_highlight_cancellation_is_cooperative_and_keeps_foreground_spans() {
        let panel = CodeEditorPanel::new(&large_initial_highlight_source(), "rs");
        let initial_spans = panel.initial_span_count();
        panel.inject_initial_highlight_fault(InitialHighlightTestFault::CancelDuringCapture);
        assert_eq!(
            await_initial_highlight_terminal(&panel),
            InitialHighlightStatus::Failed
        );
        assert_eq!(
            panel.initial_highlight_failure(),
            Some(InitialHighlightFailure::Cancelled)
        );
        assert_eq!(panel.span_count(), initial_spans);
    }

    #[test]
    fn initial_highlight_source_bound_accepts_required_fixture_and_rejects_oversize_job() {
        assert!(initial_highlight_source_is_worker_eligible(517_231));
        assert!(initial_highlight_source_is_worker_eligible(
            INITIAL_HIGHLIGHT_MAX_SOURCE_BYTES
        ));
        assert!(!initial_highlight_source_is_worker_eligible(
            INITIAL_HIGHLIGHT_MAX_SOURCE_BYTES + 1
        ));
    }

    #[test]
    fn refresh_cancels_and_releases_pending_initial_highlight_job() {
        let panel = CodeEditorPanel::new(&large_initial_highlight_source(), "rs");
        let cancel = panel
            .initial_highlight_cancel
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_ref()
            .cloned()
            .expect("large source owns a cancellation token");
        panel.refresh();
        assert!(cancel.load(Ordering::Acquire));
        assert!(panel
            .initial_highlight_source
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_none());
        assert!(panel
            .initial_highlight_job
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_none());
    }

    fn take_one_code_edit_receipt(panel: &CodeEditorPanel) -> PendingCodeEditMutationReceipt {
        let mut receipts = panel.take_pending_code_edit_receipts();
        assert_eq!(receipts.len(), 1, "expected exactly one mutation receipt");
        receipts.pop().expect("one receipt was asserted")
    }

    #[test]
    fn code_edit_producer_batches_real_text_edits_with_exact_identity_and_line_delta() {
        let panel = Arc::new(CodeEditorPanel::new("alpha", "txt"));
        panel.set_workspace_id("workspace-exact");
        panel.set_file_path("src/exact.rs");
        panel.set_host_render_pane_id(Some(Arc::from("pane-exact")));
        let factory = CodeEditorPaneFactory::from_arc(Arc::clone(&panel));
        let start = Instant::now();

        assert_eq!(
            panel.apply_text_edit_undoable("test typing", |panel| panel.insert_text("\n")),
            1
        );
        let first_receipt = take_one_code_edit_receipt(&panel);
        assert_eq!(first_receipt.pane_id.as_deref(), Some("pane-exact"));
        assert_eq!(first_receipt.workspace_id, "workspace-exact");
        assert_eq!(first_receipt.file_path, "src/exact.rs");
        factory.stage_code_edit_event_at(
            first_receipt.pane_id.as_deref().expect("captured pane"),
            &first_receipt.workspace_id,
            &first_receipt.file_path,
            first_receipt.line_delta,
            start,
        );

        assert_eq!(
            panel.apply_text_edit_undoable("test typing", |panel| panel.insert_text("\n")),
            1
        );
        let second_receipt = take_one_code_edit_receipt(&panel);
        factory.stage_code_edit_event_at(
            second_receipt.pane_id.as_deref().expect("captured pane"),
            &second_receipt.workspace_id,
            &second_receipt.file_path,
            second_receipt.line_delta,
            start + Duration::from_secs(1),
        );

        assert!(
            factory
                .take_due_code_edit_event_at(start + Duration::from_millis(2_999))
                .is_none(),
            "the second edit resets the two-second trailing edge without sleeping"
        );
        let event = factory
            .take_due_code_edit_event_at(start + Duration::from_secs(3))
            .expect("one batch becomes due exactly two seconds after the final edit");
        assert_eq!(
            event.action,
            crate::event_emitter::NativeEditorAction::CodeEdit
        );
        assert_eq!(event.workspace_id, "workspace-exact");
        assert_eq!(event.pane_id, "pane-exact");
        assert_eq!(event.actor_id, "hsk:native_editor:pane-exact");
        assert_eq!(event.payload["file_path"], "src/exact.rs");
        assert_eq!(event.payload["line_delta"], 2);
        assert!(
            factory
                .take_due_code_edit_event_at(start + Duration::from_secs(30))
                .is_none(),
            "taking the due batch consumes it exactly once"
        );
    }

    #[test]
    fn code_edit_producer_never_relabels_a_batch_after_identity_changes() {
        let panel = Arc::new(CodeEditorPanel::new("alpha", "txt"));
        panel.set_file_path("src/first.rs");
        let factory = CodeEditorPaneFactory::from_arc(Arc::clone(&panel));
        let start = Instant::now();
        factory.stage_code_edit_event_at("pane-first", "workspace-first", "src/first.rs", 1, start);

        panel.set_file_path("src/second.rs");
        factory.stage_code_edit_event_at(
            "pane-second",
            "workspace-second",
            "src/second.rs",
            -2,
            start + Duration::from_secs(1),
        );

        let first = factory
            .take_due_code_edit_event_at(start + Duration::from_secs(2))
            .expect("the closed first-identity batch becomes due first");
        assert_eq!(first.workspace_id, "workspace-first");
        assert_eq!(first.pane_id, "pane-first");
        assert_eq!(first.payload["file_path"], "src/first.rs");
        assert_eq!(first.payload["line_delta"], 1);
        assert!(
            factory
                .take_due_code_edit_event_at(start + Duration::from_millis(2_999))
                .is_none(),
            "the second identity retains its own trailing-edge deadline"
        );
        let second = factory
            .take_due_code_edit_event_at(start + Duration::from_secs(3))
            .expect("the second-identity batch becomes due on its own deadline");
        assert_eq!(second.workspace_id, "workspace-second");
        assert_eq!(second.pane_id, "pane-second");
        assert_eq!(second.payload["file_path"], "src/second.rs");
        assert_eq!(second.payload["line_delta"], -2);
    }

    #[test]
    fn code_edit_producer_covers_format_and_line_operation_product_paths() {
        let panel = CodeEditorPanel::new("alpha", "txt");
        let before_format = panel.buffer().to_string();
        let after_format = format!("{before_format}\nformatted");
        *panel
            .format_result
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((
            before_format,
            Some(after_format),
            FormatOutcome::Applied { edit_count: 1 },
        ));
        panel.drain_format_result();
        assert_eq!(
            take_one_code_edit_receipt(&panel).line_delta,
            1,
            "a successfully applied formatter result stages its exact line delta"
        );

        panel.dispatch_action(CodeEditorAction::DuplicateLine);
        assert_eq!(
            take_one_code_edit_receipt(&panel).line_delta,
            1,
            "a successful real line operation stages its exact line delta"
        );
    }

    #[test]
    fn code_edit_producer_emits_zero_receipts_for_noop_failure_and_cancel_paths() {
        let panel = CodeEditorPanel::new("alpha", "txt");

        panel.dispatch_action(CodeEditorAction::MoveLineUp);
        assert!(panel.take_pending_code_edit_receipts().is_empty());

        let unchanged = panel.buffer().to_string();
        *panel
            .format_result
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some((
            unchanged,
            None,
            FormatOutcome::LspError("formatter failed".to_owned()),
        ));
        panel.drain_format_result();
        assert!(panel.take_pending_code_edit_receipts().is_empty());

        assert!(!panel.handle_ime_event(&egui::ImeEvent::Commit(String::new())));
        assert!(panel.take_pending_code_edit_receipts().is_empty());

        let same = panel.buffer();
        panel.record_text_edit_undo(same.clone(), same, "test no-op");
        assert!(panel.take_pending_code_edit_receipts().is_empty());
    }

    #[test]
    fn code_nav_target_uses_symbol_key_file_instead_of_current_document() {
        let symbol = CodeSymbolNavProjection {
            symbol_key: "rust:src/other.rs#target".to_owned(),
            definition: Some(super::super::code_nav::CodeSymbolDefinition {
                line_start: Some(7),
                line_end: Some(8),
                ..Default::default()
            }),
            ..Default::default()
        };

        let target = code_nav_location_from_symbol(&symbol, r"C:\workspace\src\current.rs")
            .expect("symbol definition produces a navigation target");

        assert_eq!(target.path.as_deref(), Some("src/other.rs"));
        assert_eq!(target.uri, "file:///src/other.rs");
        assert_eq!(target.range.start.line, 6);
        assert_eq!(target.range.end.line, 7);
        assert!(!target.uri.contains("current.rs"));
    }

    #[test]
    fn scope_colors_come_from_theme_tokens() {
        let dark = crate::theme::HsTheme::Dark.palette().syntax;
        assert_eq!(scope_to_color(HighlightScope::Keyword, &dark), dark.keyword);
        assert_eq!(scope_to_color(HighlightScope::String, &dark), dark.string);
        assert_eq!(scope_to_color(HighlightScope::Comment, &dark), dark.comment);
        assert_eq!(scope_to_color(HighlightScope::Number, &dark), dark.number);
        assert_eq!(scope_to_color(HighlightScope::Type, &dark), dark.type_name);
        // Keyword and String differ -> at least two distinct foreground colors exist (AC-004 basis).
        assert_ne!(
            scope_to_color(HighlightScope::Keyword, &dark),
            scope_to_color(HighlightScope::String, &dark),
        );
    }

    #[test]
    fn code_text_undo_batcher_replaces_tail_inside_500ms_window() {
        let mut batcher = CodeTextUndoBatcher::default();
        let start = Instant::now();

        let (first_before, first_replace) =
            batcher.observe_edit(TextBuffer::new("before first"), start);
        assert!(
            !first_replace,
            "the first edit in a batch pushes a fresh undo entry"
        );
        assert_eq!(first_before.to_string(), "before first");

        let (second_before, second_replace) = batcher.observe_edit(
            TextBuffer::new("before second"),
            start + Duration::from_millis(250),
        );
        assert!(
            second_replace,
            "an edit inside the 500ms code typing window replaces the local undo tail"
        );
        assert_eq!(
            second_before.to_string(),
            "before first",
            "coalesced code typing keeps the first pre-burst snapshot"
        );

        let (third_before, third_replace) = batcher.observe_edit(
            TextBuffer::new("before third"),
            start + Duration::from_millis(900),
        );
        assert!(
            !third_replace,
            "an edit outside the 500ms window starts a fresh code undo entry"
        );
        assert_eq!(third_before.to_string(), "before third");
    }

    #[test]
    fn span_window_keeps_long_parent_after_ended_child_span() {
        let spans = vec![
            HighlightSpan {
                byte_range: 0..10_000,
                scope: HighlightScope::String,
            },
            HighlightSpan {
                byte_range: 100..102,
                scope: HighlightScope::Other,
            },
            HighlightSpan {
                byte_range: 1_100..1_112,
                scope: HighlightScope::Keyword,
            },
            HighlightSpan {
                byte_range: 20_000..20_010,
                scope: HighlightScope::Comment,
            },
        ];
        let window = HighlightSpanWindow::from_spans(spans);
        let overlaps: Vec<_> = window.overlapping(1_000, 1_010).collect();

        assert_eq!(
            overlaps.len(),
            1,
            "a short child span ending before the window must not hide the long parent span"
        );
        assert_eq!(overlaps[0].byte_range, 0..10_000);
        assert_eq!(overlaps[0].scope, HighlightScope::String);
    }

    /// WP-KERNEL-012 wave-6 (S6 item 3): a Custom syntax-palette override changes the color the RENDER PATH
    /// resolves for a scope. `resolve_highlight_color` is the exact resolver the panel-body draw paths call
    /// for every highlighted run, so this proves a Custom swatch edit repaints the running editor (the
    /// MT-072 typed follow-up, now wired). With no palette set the resolver falls back to the theme token
    /// (unchanged behavior); with a Custom override it returns the override color.
    #[test]
    fn custom_palette_override_changes_render_path_color() {
        use crate::workspace_settings::{SyntaxPalette, SyntaxPaletteMode};
        let dark = crate::theme::HsTheme::Dark.palette().syntax;
        let panel = CodeEditorPanel::new("fn main() {}", "rs");

        // Default (no live palette installed): the render-path resolver uses the theme keyword token.
        assert_eq!(
            panel.resolve_highlight_color(HighlightScope::Keyword, &dark),
            dark.keyword,
            "no palette installed -> render path uses the theme keyword color"
        );

        // Install a Custom palette overriding Keyword to a distinct sRGBA color.
        let mut palette = SyntaxPalette {
            mode: SyntaxPaletteMode::Custom,
            custom: Default::default(),
        };
        palette.set_custom(HighlightScope::Keyword.scope_key(), [200, 30, 30, 255]);
        panel.set_syntax_palette(palette);

        let resolved = panel.resolve_highlight_color(HighlightScope::Keyword, &dark);
        assert_eq!(
            resolved,
            egui::Color32::from_rgba_unmultiplied(200, 30, 30, 255),
            "Custom override is applied in the render-path resolver"
        );
        assert_ne!(
            resolved, dark.keyword,
            "the Custom override actually CHANGES the render-path keyword color vs the theme token"
        );
        // A non-overridden scope in Custom mode falls back to a concrete color (never missing/panics).
        let _ = panel.resolve_highlight_color(HighlightScope::Comment, &dark);
    }

    /// MT-108 (MT-004 residual, RISK-004): stepping to a match deep in the document scrolls it to just
    /// BELOW the pinned find bar, not to the very top of the viewport where the floating bar would
    /// occlude it. The requested scroll offset is `line*line_height - find_bar_inset`, strictly less
    /// than `line*line_height` (the naive scroll-to-top), which is the whole point of the inset.
    #[test]
    fn find_step_scrolls_current_match_below_the_find_bar() {
        let mut doc = String::new();
        for i in 0..200 {
            doc.push_str(&format!("line {i}\n"));
        }
        doc.push_str("needle here\n"); // the only match, on line 200
        let panel = CodeEditorPanel::new(&doc, "txt");
        // Pin a known measured line height so the offset math is deterministic (no render needed).
        *panel.line_height_px.lock().unwrap() = Some(15.0);

        panel.open_find(false);
        panel.set_find_query("needle");
        let state = panel.find_state().expect("bar open");
        assert_eq!(state.matches.len(), 1, "one 'needle' match");
        assert_eq!(state.matches[0].line, 200, "the match is on line 200");

        // Step to the current match (single match -> stays index 0 but still requests the scroll).
        panel.next_match();

        let offset = panel
            .pending_scroll_offset
            .lock()
            .unwrap()
            .expect("stepping to a match requested a scroll");
        let lh = 15.0_f32;
        let top_of_line = 200.0_f32 * lh;
        let inset =
            FIND_BAR_TOP_MARGIN_PX + FIND_BAR_HEIGHT_SINGLE_PX + FIND_BAR_MATCH_REVEAL_GAP_PX;
        assert!(
            (offset - (top_of_line - inset)).abs() < 0.01,
            "scroll offset insets by the find-bar height: got {offset}, want {}",
            top_of_line - inset
        );
        assert!(
            offset < top_of_line,
            "the inset actually pushes the match below the bar (offset {offset} < scroll-to-top \
             {top_of_line})"
        );
    }

    /// MT-108 (MT-006 residual): with a fold ACTIVE above the navigation target, `navigate_to_line`
    /// must land on the FOLD-ADJUSTED visible row (a collapsed region above the target shifts its
    /// visible row up), not the raw buffer line. This is the outline-click / go-to-line landing-row
    /// behavior when folds are present.
    #[test]
    fn navigate_to_line_lands_on_fold_adjusted_visible_row_when_a_fold_is_active() {
        let mut src = String::from("fn top() {\n");
        for i in 0..8 {
            src.push_str(&format!("    let _x{i} = {i};\n"));
        }
        src.push_str("}\n"); // top()'s body is a foldable region
        for i in 0..5 {
            src.push_str(&format!("// gap {i}\n"));
        }
        src.push_str("fn target() {}\n"); // the navigation target (below the foldable region)
        let panel = CodeEditorPanel::new(&src, "rs");
        *panel.line_height_px.lock().unwrap() = Some(10.0);

        let target_line = panel.with_buffer(|b| b.len_lines()) - 1;

        // Baseline: nothing folded -> visible row == buffer line, scroll offset == target_line * lh.
        assert_eq!(
            panel.buffer_line_to_visible_line(target_line),
            target_line,
            "with nothing folded the visible row equals the buffer line"
        );
        panel.navigate_to_line(target_line);
        let unfolded_offset = panel
            .pending_scroll_offset
            .lock()
            .unwrap()
            .expect("navigate requested a scroll");
        assert!((unfolded_offset - target_line as f32 * 10.0).abs() < 0.01);

        // Fold the top() function body (a region enclosing line 1). This hides its inner lines.
        assert!(
            panel.fold_at_line(1),
            "the top() function body is foldable and is now folded"
        );

        // Now the target's visible row is SHIFTED UP by the hidden lines.
        let visible = panel.buffer_line_to_visible_line(target_line);
        assert!(
            visible < target_line,
            "a fold above the target shifts its visible landing row up (visible {visible} < buffer \
             line {target_line})"
        );

        // Navigating again lands on the fold-adjusted visible row (a lower scroll offset).
        panel.navigate_to_line(target_line);
        let folded_offset = panel
            .pending_scroll_offset
            .lock()
            .unwrap()
            .expect("navigate requested a scroll");
        assert!(
            (folded_offset - visible as f32 * 10.0).abs() < 0.01,
            "the landing scroll offset equals the fold-adjusted visible row * line height"
        );
        assert!(
            folded_offset < unfolded_offset,
            "folding above the target lowers the landing scroll offset ({folded_offset} < \
             {unfolded_offset})"
        );
    }

    /// WP-KERNEL-012 wave-6 (S6 item 3): `set_font_size` changes the panel's LIVE font size + invalidates
    /// the measured-metric caches, so the next measurement re-measures at the new size. This is the
    /// non-rendered half of the font-size proof; the rendered row-height proof lives below as a panel.rs
    /// unit test so test-only row-height internals stay private to the module.
    #[test]
    fn set_font_size_updates_slot_and_invalidates_metric_caches() {
        let panel = CodeEditorPanel::new("fn main() {}", "rs");
        assert_eq!(
            panel.font_size(),
            MONO_FONT_SIZE,
            "default font size is the built-in MONO_FONT_SIZE"
        );
        // Seed the caches with fake measured values, then change the size and confirm they are cleared.
        *panel.line_height_px.lock().unwrap() = Some(15.0);
        *panel.glyph_width_px.lock().unwrap() = Some(8.0);
        panel.set_font_size(28.0);
        assert_eq!(
            panel.font_size(),
            28.0,
            "the live font size updated to 28pt"
        );
        assert!(
            panel.line_height_px.lock().unwrap().is_none(),
            "changing the font size invalidated the line-height cache (next frame re-measures)"
        );
        assert!(
            panel.measured_glyph_width().is_none(),
            "changing the font size invalidated the glyph-width cache"
        );
        // Out-of-range is clamped to the settings range (6..=48).
        panel.set_font_size(999.0);
        assert_eq!(panel.font_size(), 48.0, "font size clamped to the 48pt max");
        panel.set_font_size(1.0);
        assert_eq!(panel.font_size(), 6.0, "font size clamped to the 6pt min");
    }

    /// WP-KERNEL-012 wave-6 (S6 item 3): the LIVE editor-font-size effect is proven through the RENDER
    /// path without exposing line-height internals as product API. The test drives the real
    /// [`CodeEditorPanel::show`] path in an egui_kittest harness and inspects the module-private cached
    /// line-height/glyph-width measurements before and after live font-size changes.
    #[test]
    fn font_size_change_resizes_measured_row_height_through_render_path() {
        use std::sync::Arc;

        use egui_kittest::Harness;

        const SNIPPET: &str = "\
fn main() {
    let name = \"world\";
    // greet
    println!(\"hi {name}\");
}";

        let panel = Arc::new(CodeEditorPanel::new(SNIPPET, "rs"));
        let panel_for_harness = Arc::clone(&panel);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(640.0, 300.0))
            .build_ui(move |ui| {
                panel_for_harness.show(ui);
            });

        harness.run();
        let h_default = panel
            .line_height_px
            .lock()
            .unwrap()
            .expect("row height measured after the first render at the default font size");
        let w_default = panel
            .glyph_width_px
            .lock()
            .unwrap()
            .expect("glyph advance measured at the default font size");
        let gutter_default = panel
            .last_gutter_geometry
            .lock()
            .unwrap()
            .expect("gutter geometry captured at the default font size");
        assert_eq!(gutter_default.font_size, MONO_FONT_SIZE);
        assert!(
            h_default > 0.0 && w_default > 0.0,
            "sane default metrics: row_height={h_default}, glyph_width={w_default}"
        );

        panel.set_font_size(28.0);
        harness.run();
        let h_big = panel
            .line_height_px
            .lock()
            .unwrap()
            .expect("row height re-measured at 28pt");
        let w_big = panel
            .glyph_width_px
            .lock()
            .unwrap()
            .expect("glyph advance re-measured at 28pt");
        let gutter_big = panel
            .last_gutter_geometry
            .lock()
            .unwrap()
            .expect("gutter geometry refreshed at 28pt");
        assert_eq!(
            gutter_big.font_size, 28.0,
            "the gutter line-number/fold glyph font follows the live editor font size"
        );
        assert!(
            h_big > h_default + 1.0,
            "S6 item 3: a larger font size GREW the measured row height ({h_default} -> {h_big})"
        );
        assert!(
            w_big > w_default,
            "S6 item 3: a larger font size grew the measured glyph advance ({w_default} -> {w_big})"
        );

        // Exercise the supported maximum through the same mounted render path. The foldable Rust
        // snippet must receive a strip whose measured width includes the live, expanded fold column;
        // otherwise a 48pt triangle can paint into the line-number column even though both glyphs use
        // the correct font size.
        panel.set_font_size(48.0);
        harness.run();
        let gutter_max = panel
            .last_gutter_geometry
            .lock()
            .unwrap()
            .expect("gutter geometry refreshed at the supported 48pt maximum");
        let gutter_max_rect = panel
            .last_gutter_rect
            .lock()
            .unwrap()
            .expect("gutter strip captured at the supported 48pt maximum");
        let total_lines = panel.buffer.lock().unwrap().len_lines();
        let expected_width =
            Gutter::width_for(total_lines, gutter_max.char_width, &panel.gutter_config());
        assert_eq!(gutter_max.font_size, 48.0);
        assert!(
            crate::code_editor::gutter::fold_column_width(gutter_max.char_width) > 14.0,
            "48pt render expands the fold column beyond its 14px minimum"
        );
        assert!(
            (gutter_max_rect.width() - expected_width).abs() < 0.5,
            "mounted 48pt gutter reserves the same live width used by fold glyph and hit geometry: \
             rect={} expected={expected_width}",
            gutter_max_rect.width()
        );

        panel.set_font_size(8.0);
        harness.run();
        let h_tiny = panel
            .line_height_px
            .lock()
            .unwrap()
            .expect("row height re-measured at 8pt");
        let gutter_tiny = panel
            .last_gutter_geometry
            .lock()
            .unwrap()
            .expect("gutter geometry refreshed at 8pt");
        assert_eq!(gutter_tiny.font_size, 8.0);
        assert!(
            h_tiny < h_default,
            "S6 item 3: a smaller font size SHRANK the measured row height ({h_default} -> {h_tiny})"
        );
    }

    /// WP-KERNEL-012 MT-035 wave-7: the LIVE line-height MULTIPLIER respaces the mounted editor's rows.
    /// Driven through the REAL render path (like the font-size proof) and inspected via the module-private
    /// cached row-height measurement: a 1.8x multiplier makes the computed row height ~1.8x taller, and
    /// resetting to 1.0 restores the natural single-spaced height — a real, cache-invalidated respacing,
    /// NOT a dead toggle.
    #[test]
    fn line_height_multiplier_respaces_measured_row_height_through_render_path() {
        use std::sync::Arc;

        use egui_kittest::Harness;

        let panel = Arc::new(CodeEditorPanel::new("fn main() {\n    let x = 1;\n}", "rs"));
        let panel_for_harness = Arc::clone(&panel);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(640.0, 300.0))
            .build_ui(move |ui| {
                panel_for_harness.show(ui);
            });

        harness.run();
        let h_single = panel
            .line_height_px
            .lock()
            .unwrap()
            .expect("row height measured at the default 1.0 multiplier");
        assert!(h_single > 0.0, "sane single-spaced row height: {h_single}");
        assert!(
            (panel.line_height_multiplier() - 1.0).abs() < f32::EPSILON,
            "default multiplier is 1.0 (single-spaced)"
        );

        panel.set_line_height(1.8);
        harness.run();
        let h_scaled = panel
            .line_height_px
            .lock()
            .unwrap()
            .expect("row height re-measured at the 1.8 multiplier");
        assert!(
            (panel.line_height_multiplier() - 1.8).abs() < 1e-4,
            "the multiplier reached the panel"
        );
        assert!(
            (h_scaled - h_single * 1.8).abs() < 0.5,
            "MT-035 wave-7: a 1.8x line-height GREW the computed row height ~1.8x ({h_single} -> \
             {h_scaled}, expected ~{})",
            h_single * 1.8
        );

        // The OTHER direction: back to single-spaced restores the natural row height.
        panel.set_line_height(1.0);
        harness.run();
        let h_restored = panel
            .line_height_px
            .lock()
            .unwrap()
            .expect("row height re-measured back at 1.0");
        assert!(
            (h_restored - h_single).abs() < 0.5,
            "MT-035 wave-7: resetting to 1.0 restored the single-spaced height ({h_scaled} -> \
             {h_restored}, expected ~{h_single})"
        );
    }

    /// WP-KERNEL-012 MT-035 wave-7: the bracket-matching toggle GATES the matched-bracket computation the
    /// render path highlights. With a caret next to `(`, the enabled toggle computes the matching `)`
    /// position; disabling it yields `None` (no highlight); re-enabling restores it — proving both
    /// directions drive a real feature, not a dead toggle.
    #[test]
    fn bracket_matching_toggle_gates_match_computation() {
        // "()" — caret byte 0 sits immediately BEFORE the '(' (VS Code before-open adjacency).
        let panel = CodeEditorPanel::new("()", "rs");
        assert!(
            panel.bracket_matching_enabled(),
            "bracket matching defaults on (always-on pre-toggle behavior)"
        );
        assert_eq!(
            panel.matching_bracket_at(0),
            Some((0, 1)),
            "MT-035 wave-7: enabled -> the caret next to '(' matches the ')' at byte 1"
        );

        panel.set_bracket_matching_enabled(false);
        assert_eq!(
            panel.matching_bracket_at(0),
            None,
            "MT-035 wave-7: disabled -> no matching bracket is computed (no highlight)"
        );

        panel.set_bracket_matching_enabled(true);
        assert_eq!(
            panel.matching_bracket_at(0),
            Some((0, 1)),
            "MT-035 wave-7: re-enabled -> the match is computed again"
        );
    }

    /// WP-KERNEL-012 MT-035 wave-7: the indent-guides toggle GATES the guide computation the render path
    /// paints. An indented line reports its indent-guide count when enabled and `0` when disabled — both
    /// directions drive a real feature, not a dead toggle.
    #[test]
    fn indent_guides_toggle_gates_guide_count() {
        // Line index 1 ("\tlet x = 1;") is indented one tab -> one indent level -> one guide.
        let panel = CodeEditorPanel::new("fn main() {\n\tlet x = 1;\n}", "rs");
        assert!(
            panel.indent_guides_enabled(),
            "indent guides default on (always-on pre-toggle behavior)"
        );
        let guides_on = panel.indent_guide_count_for_line(1);
        assert!(
            guides_on >= 1,
            "MT-035 wave-7: enabled -> the indented line exposes >=1 indent guide (got {guides_on})"
        );

        panel.set_indent_guides_enabled(false);
        assert_eq!(
            panel.indent_guide_count_for_line(1),
            0,
            "MT-035 wave-7: disabled -> the panel exposes no indent guides"
        );

        panel.set_indent_guides_enabled(true);
        assert_eq!(
            panel.indent_guide_count_for_line(1),
            guides_on,
            "MT-035 wave-7: re-enabled -> the guide count is computed again"
        );
    }

    #[test]
    fn panel_highlights_rust_on_construction() {
        let panel = CodeEditorPanel::new("fn main() { let x = 1; }", "rs");
        assert!(
            panel
                .spans()
                .iter()
                .any(|s| s.scope == HighlightScope::Keyword),
            "constructed rust panel carries keyword spans"
        );
    }

    #[test]
    fn unknown_extension_panel_has_no_spans_but_renders() {
        let panel = CodeEditorPanel::new("plain text\nsecond line", "txt");
        assert!(
            panel.spans().is_empty(),
            "no grammar -> no spans (plain text)"
        );
        // Render it once to prove no panic on the unhighlighted path.
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
        });
    }

    #[test]
    fn instance_suffix_disambiguates_author_ids() {
        let a = CodeEditorPanel::with_instance("x", "rs", "left");
        let b = CodeEditorPanel::with_instance("y", "rs", "right");
        assert_eq!(a.container_author_id(), "code_editor_panel#left");
        assert_eq!(a.scroll_author_id(), "code_editor_scroll_area#left");
        assert_eq!(b.container_author_id(), "code_editor_panel#right");
        assert_ne!(a.container_author_id(), b.container_author_id());
        assert_ne!(a.scroll_author_id(), b.scroll_author_id());
        assert_ne!(a.text_author_id(), b.text_author_id());
        // The default panel uses the bare MT-contract ids (AC-004/AC-005).
        let d = CodeEditorPanel::new("z", "rs");
        assert_eq!(d.container_author_id(), CODE_EDITOR_PANEL_AUTHOR_ID);
        assert_eq!(d.scroll_author_id(), CODE_EDITOR_SCROLL_AREA_AUTHOR_ID);
        assert_eq!(d.text_author_id(), CODE_EDITOR_TEXT_AUTHOR_ID);
        assert_eq!(d.text_author_id(), "editor.code.text");
    }

    #[test]
    fn large_document_render_is_virtualized() {
        // 5000 lines -> the panel must paint only the visible window (a few dozen lines), not all
        // 5000, after a frame runs (MT-002 virtualization replaces the MT-001 hard render cap).
        let big = "x\n".repeat(5000);
        let panel = CodeEditorPanel::new(&big, "rs");
        assert!(panel.buffer().len_lines() > 1000);
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
        });
        let stats = panel.perf_stats();
        assert_eq!(
            stats.buffer_len_lines, 5001,
            "whole document line count reported"
        );
        // The painted window must be strictly fewer lines than the whole document — that is
        // virtualization. (On a default headless egui Context the CentralPanel viewport is large, so
        // the absolute count depends on viewport height; the load-bearing fact is `painted < total`.
        // The fixed-window kittest screenshot proof asserts the tighter visible-window bound.)
        assert!(
            stats.frame_lines_rendered > 0 && stats.frame_lines_rendered < stats.buffer_len_lines,
            "virtualized: fewer lines painted than the whole doc (got {} of {})",
            stats.frame_lines_rendered,
            stats.buffer_len_lines
        );
    }

    #[test]
    fn highlight_cache_recomputes_only_on_version_change() {
        // The cache is filled at construction (version 1). Calling spans() twice without a refresh is
        // a cache hit (same version); refresh() bumps the version and recomputes.
        let panel = CodeEditorPanel::new("fn main() {}", "rs");
        let v0 = panel.buffer_version.load(Ordering::Relaxed);
        let _ = panel.spans();
        assert_eq!(
            panel.buffer_version.load(Ordering::Relaxed),
            v0,
            "spans() alone does not bump the version"
        );
        panel.refresh();
        assert_eq!(
            panel.buffer_version.load(Ordering::Relaxed),
            v0 + 1,
            "refresh bumps the buffer version (RISK-002)"
        );
        // Cache is re-filled at the new version.
        let cached_version = panel
            .highlight_cache
            .lock()
            .unwrap()
            .as_ref()
            .map(|(_, v)| *v);
        assert_eq!(
            cached_version,
            Some(v0 + 1),
            "cache re-filled at the bumped version"
        );
    }

    #[test]
    fn host_incarnation_is_monotonic_and_never_reuses_panel_identity() {
        let mut previous = 0;
        let mut seen = std::collections::HashSet::new();
        // A bounded sample is sufficient to prove the constructor uses the monotonic allocator;
        // panel construction performs full syntax/highlight setup, so keep this regression cheap.
        for _ in 0..64 {
            let panel = CodeEditorPanel::new("fn identity_probe() {}", "rs");
            let incarnation = panel.host_incarnation();
            assert!(
                incarnation > previous,
                "panel incarnation must increase monotonically: previous={previous}, current={incarnation}"
            );
            assert!(
                seen.insert(incarnation),
                "panel incarnation {incarnation} was reused"
            );
            previous = incarnation;
        }
    }

    #[test]
    fn note_refs_late_delivery_cannot_overwrite_newer_request() {
        let panel = CodeEditorPanel::new("fn a() {}", "rs");
        panel.set_workspace_id("ws-b");
        let request_a = NoteRefsRequestStamp {
            workspace_id: "ws-a".to_owned(),
            symbol: "src/a.rs#A".to_owned(),
            generation: 1,
        };
        let request_b = NoteRefsRequestStamp {
            workspace_id: "ws-b".to_owned(),
            symbol: "src/b.rs#B".to_owned(),
            generation: 2,
        };
        *panel
            .note_refs_active_request
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(request_b.clone());
        *panel
            .note_refs_state
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = NoteRefsState::Loading;
        panel
            .note_refs_result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .extend([
                NoteRefsDelivery {
                    stamp: request_b,
                    symbol_key: Some("rust:src/b.rs#B".to_owned()),
                    state: NoteRefsState::Loaded(vec![crate::interop::cross_ref::NoteRef {
                        block_id: "B".to_owned(),
                        document_id: "B".to_owned(),
                        document_title: "current B".to_owned(),
                        excerpt: String::new(),
                    }]),
                },
                NoteRefsDelivery {
                    stamp: request_a,
                    symbol_key: Some("rust:src/a.rs#A".to_owned()),
                    state: NoteRefsState::Loaded(vec![crate::interop::cross_ref::NoteRef {
                        block_id: "A".to_owned(),
                        document_id: "A".to_owned(),
                        document_title: "late A".to_owned(),
                        excerpt: String::new(),
                    }]),
                },
            ]);
        panel.drain_note_refs();
        assert!(
            matches!(panel.note_refs_state(), NoteRefsState::Loaded(notes) if notes[0].document_id == "B"),
            "a late A queued after B must not overwrite or discard the exact active B delivery"
        );
    }

    fn stage_note_refs_delivery_before_backend_replacement(
        panel: &CodeEditorPanel,
    ) -> NoteRefsRequestStamp {
        let stamp = NoteRefsRequestStamp {
            workspace_id: panel.workspace_id(),
            symbol: "src/stale.rs#Stale".to_owned(),
            generation: panel.note_refs_generation.load(Ordering::Relaxed),
        };
        *panel
            .note_refs_active_request
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(stamp.clone());
        *panel
            .note_refs_state
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = NoteRefsState::Loading;
        stamp
    }

    fn deliver_stale_note_refs_after_backend_replacement(
        panel: &CodeEditorPanel,
        stamp: NoteRefsRequestStamp,
    ) {
        panel
            .note_refs_result
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .push(NoteRefsDelivery {
                stamp,
                symbol_key: Some("rust:src/stale.rs#Stale".to_owned()),
                state: NoteRefsState::Loaded(vec![crate::interop::cross_ref::NoteRef {
                    block_id: "BLK-STALE".to_owned(),
                    document_id: "DOC-STALE".to_owned(),
                    document_title: "stale backend result".to_owned(),
                    excerpt: String::new(),
                }]),
            });
        panel.drain_note_refs();
        assert_eq!(panel.note_refs_state(), NoteRefsState::Idle);
        assert!(
            panel
                .note_refs_active_request
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_none(),
            "backend replacement must revoke ownership of the old NoteRefs request"
        );
        assert!(panel.note_refs_focused_symbol().is_none());
    }

    #[test]
    fn replacing_code_nav_client_fences_old_note_refs_delivery() {
        let panel = CodeEditorPanel::new("fn stale() {}", "rs");
        panel.set_workspace_id("ws-note-refs-client-swap");
        let stamp = stage_note_refs_delivery_before_backend_replacement(&panel);
        let generation_before = panel.note_refs_generation.load(Ordering::Relaxed);

        panel.set_code_nav_client(CodeNavClient::new("http://replacement.invalid"));

        assert!(
            panel.note_refs_generation.load(Ordering::Relaxed) > generation_before,
            "CodeNavClient replacement must advance the NoteRefs ownership generation"
        );
        deliver_stale_note_refs_after_backend_replacement(&panel, stamp);
    }

    #[test]
    fn replacing_find_notes_backend_fences_old_note_refs_delivery() {
        let panel = CodeEditorPanel::new("fn stale() {}", "rs");
        panel.set_workspace_id("ws-note-refs-backend-swap");
        let stamp = stage_note_refs_delivery_before_backend_replacement(&panel);
        let generation_before = panel.note_refs_generation.load(Ordering::Relaxed);
        let replacement: Arc<dyn FindNotesSearch> =
            Arc::new(FindNotesHttp::new("http://replacement.invalid"));

        panel.set_find_notes_backend(replacement);

        assert!(
            panel.note_refs_generation.load(Ordering::Relaxed) > generation_before,
            "FindNotes backend replacement must advance the NoteRefs ownership generation"
        );
        deliver_stale_note_refs_after_backend_replacement(&panel, stamp);
    }

    #[test]
    fn note_ref_open_is_retained_until_bus_lock_is_available() {
        let panel = CodeEditorPanel::new("", "rs");
        *panel
            .pending_note_ref_open
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some("DOC-RETRY".to_owned());
        let ctx = egui::Context::default();
        let bus = InteractionBus::get_or_init(&ctx);
        let guard = bus.lock().expect("hold bus to force one contended frame");
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.render_note_refs_panel_into(ui));
        });
        assert_eq!(
            panel
                .pending_note_ref_open
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .as_deref(),
            Some("DOC-RETRY"),
            "try_lock contention must retain, not drop, the operator action"
        );
        drop(guard);
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.render_note_refs_panel_into(ui));
        });
        assert!(
            panel
                .pending_note_ref_open
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .is_none(),
            "the retained action clears only after successful bus delivery"
        );
        assert_eq!(
            bus.lock()
                .expect("read delivered bus action")
                .take_pending_navigation()
                .as_deref(),
            Some("DOC-RETRY")
        );
    }
}
