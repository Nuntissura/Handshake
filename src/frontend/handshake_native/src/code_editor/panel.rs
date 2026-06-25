//! egui widget that renders the native code editor panel (WP-KERNEL-012 MT-001 + MT-002).
//!
//! [`CodeEditorPanel`] owns a [`TextBuffer`] + a [`Highlighter`] and paints the visible lines with
//! per-scope theme colors. It exposes three stable AccessKit nodes a swarm agent addresses:
//! - an OUTER `Role::GenericContainer` node with `author_id = "code_editor_panel"` (the panel frame),
//! - a `Role::ScrollView` node with `author_id = "code_editor_scroll_area"` (the virtualized scroll
//!   region — MT-002), and
//! - an INNER `Role::TextInput` node with `author_id = "code_editor_text"` (the editable text area),
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

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::accessibility::editor_action_registry::{
    CodeDispatch, EditorActionRegistry, EditorActionState, PaneType as EditorPaneType,
    RegistrationHandle, CODE_ACTION_CATALOG,
};
use crate::interop::cross_ref::{
    find_notes_with, FindNotesHttp, FindNotesSearch, SymbolDwellTracker,
};
use crate::interop::InteractionBus;
use crate::pane_registry::{PaneFactory, PaneId, PaneRenderContext, PaneType};
use crate::theme::HsSyntaxTokens;

use super::note_refs_panel::{render_note_refs_panel, NoteRefsState};

use super::buffer::TextBuffer;
use super::code_nav::{
    staleness_marker_for, CodeNavCache, CodeNavClient, CodeSymbolNavProjection,
    CodeSymbolReferencesResponse, CompletionItem, COMPLETION_DEBOUNCE_MS, HOVER_DWELL_MS,
    SYMBOL_LOOKUP_LIMIT,
};
use super::cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet,
    MAX_ACCESSKIT_CURSORS,
};
use super::editor_view::{
    CompletionOutcome, CompletionPopup, CompletionState, HoverOutcome, HoverState, HoverTooltip,
};
use super::formatting::{self, FormatOutcome};
// MT-051 line-edit buffer transforms: the dispatch arms for ToggleComment / DuplicateLine / MoveLine /
// DeleteLine / Indent / Dedent / InsertTab call into this module (pure TextBuffer + CursorSet transforms).
use super::line_ops;
use super::lsp_client::{LspClient, PublishedDiagnostics};
use super::signature_help::{
    active_parameter_from_commas, render_signature_popup, SignatureHelpState,
};
use super::rename::{
    self, PreviewAction, RenameApplyReport, RenameState, WorkspaceEditPreview,
};
use super::code_actions::{self, AppliedAction, CodeActionController, MenuAction};
use super::find_replace::{FindEngine, FindQuery, Match};
use super::breakpoints::{BreakpointAction, BreakpointEvent, BreakpointSet};
use super::folding::{FoldProvider, FoldSet};
use super::gutter::{
    DiagnosticSeverity, Gutter, GutterConfig, GutterGeometry, GutterMarker, GutterMarkerKind,
    GutterResponse,
};
use super::highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry};
use super::jump_history::{JumpEntry, JumpHistory};
use super::keymap::{CodeEditorAction, KeyChord, Keymap};
use super::navigation::{next_diagnostic, prev_diagnostic, BufferPosition};
use super::keymap_settings::{keymap_settings_path, KeymapSettings};
use super::minimap::Minimap;
use super::outline::{OutlineItem, OutlineProvider};
use super::cursor::MoveDir;
// MT-054 editor-chrome decorations: bracket match / pair-colorize + indent-guide geometry, and the
// word-wrap VisualRow layout math. Pure over the buffer; the panel paint path consumes them.
use super::render_decorations::{
    bracket_pair_colors, find_matching_bracket, indent_guide_x, indent_level_of, BracketMatch,
};
use super::word_wrap::{count_visual_rows_for_line, layout_visual_rows, VisualRow, WrapConfig};

/// The MT-contract author_id for the outer panel container (AC-005: Role::GenericContainer).
pub const CODE_EDITOR_PANEL_AUTHOR_ID: &str = "code_editor_panel";
/// The MT-002 author_id for the virtualized scroll region (AC-004: Role::ScrollView).
pub const CODE_EDITOR_SCROLL_AREA_AUTHOR_ID: &str = "code_editor_scroll_area";
/// The MT-contract author_id for the inner editable text area (AC-005: Role::TextInput).
pub const CODE_EDITOR_TEXT_AUTHOR_ID: &str = "code_editor_text";
/// The MT-003 author_id PREFIX for each multi-cursor node (AC-004: `code_editor_cursor_{n}`). Cursor
/// `n` (sorted index) gets `code_editor_cursor_{n}` with accesskit `Role::Caret` (the field-correct
/// caret role in accesskit 0.21 — the contract's `Role::TextCursor` does not exist there). Only the
/// first [`MAX_ACCESSKIT_CURSORS`] cursors are surfaced (RISK-004 / MC-004).
pub const CODE_EDITOR_CURSOR_AUTHOR_PREFIX: &str = "code_editor_cursor_";

/// MT-004 find/replace author_ids. The find input is `code_editor_find_bar` (the MT contract names
/// `Role::SearchBox`, which does NOT exist in accesskit 0.21 — `Role::SearchInput` is the field-correct
/// equivalent; see `emit_find_bar_nodes` for the documented deviation). The replace input is
/// `code_editor_replace_bar` (`Role::TextInput`) and the Next button is `code_editor_find_next`
/// (`Role::Button`). The Prev button reuses the same pattern with a fresh slot.
pub const CODE_EDITOR_FIND_BAR_AUTHOR_ID: &str = "code_editor_find_bar";
pub const CODE_EDITOR_REPLACE_BAR_AUTHOR_ID: &str = "code_editor_replace_bar";
pub const CODE_EDITOR_FIND_NEXT_AUTHOR_ID: &str = "code_editor_find_next";
pub const CODE_EDITOR_FIND_PREV_AUTHOR_ID: &str = "code_editor_find_prev";

/// The MT-005 author_id PREFIX for each foldable-region node (AC-005: `code_editor_fold_{start_line}`).
/// Region starting on buffer line `L` gets `code_editor_fold_{L}` with accesskit `Role::TreeItem` and
/// an `Action::Expand` (when folded) or `Action::Collapse` (when unfolded) action a swarm agent
/// dispatches to fold/unfold by id. Only the foldable regions inside the painted window are surfaced
/// (capped — RISK-001) so a 1000-fold file does not emit 1000 nodes per frame.
pub const CODE_EDITOR_FOLD_AUTHOR_PREFIX: &str = "code_editor_fold_";

/// MT-006 navigation-aid author_ids (AC-003/004/005). The minimap node is `code_editor_minimap`
/// (`Role::ScrollBar` — clicking scrolls; the role exists in accesskit 0.21.1, no fallback needed); the
/// outline tree is `code_editor_outline` (`Role::Tree`); the go-to-line input is `code_editor_goto_line`
/// (`Role::TextInput`). All three roles named in the MT contract exist in accesskit 0.21.1 (verified
/// against the locked source), so unlike the MT-003 TextCursor / MT-004 SearchBox cases no role fallback
/// is required for this MT.
pub const CODE_EDITOR_MINIMAP_AUTHOR_ID: &str = "code_editor_minimap";
pub const CODE_EDITOR_OUTLINE_AUTHOR_ID: &str = "code_editor_outline";
pub const CODE_EDITOR_GOTO_LINE_AUTHOR_ID: &str = "code_editor_goto_line";

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
pub const CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX: &str = "code_editor_breakpoint_";
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

/// The fixed AccessKit `NodeId` band the per-fold `Role::TreeItem` nodes occupy for the default panel
/// (300..300+MAX_ACCESSKIT_FOLDS), disjoint from the find-bar band (280..283), the cursor band
/// (210..274), and the container/scroll/text band (200/201/202).
const PANEL_FOLD_NODE_ID_BASE: u64 = 300;

/// MT-006 navigation-aid fixed AccessKit `NodeId`s for the default (single-instance) panel. A fresh
/// band (370..372) ABOVE the fold band (300..363) so they never collide with the container/scroll/text
/// (200/201/202), cursor (210..274), find-bar (280..283), or fold nodes. Multi-instance panels hash the
/// suffixed author_id instead (RISK-004), the same scheme every other panel node uses.
const PANEL_MINIMAP_NODE_ID: u64 = 370;
const PANEL_OUTLINE_NODE_ID: u64 = 371;
const PANEL_GOTO_LINE_NODE_ID: u64 = 372;

/// MT-053 fixed AccessKit `NodeId`s for the default (single-instance) panel. A fresh band (700..702)
/// ABOVE the MT-010 command band (600..600+N≈660), disjoint from the container/scroll/text (200/201/202),
/// cursor (210..274), find-bar (280..283), fold (300..363), nav (370..372), gutter (400/410../480..), and
/// command (600..) bands. The symbol-palette dialog (the modal window scope), the palette list container,
/// and the search input get fixed ids; the sticky band container gets a fixed id too. Per-row + per-header
/// nodes are DYNAMIC (hashed id space). Multi-instance panels hash the suffixed author_id instead
/// (RISK-004), the same scheme every other panel node uses.
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
/// cursor (210..274), find-bar (280..283), fold (300..363), and nav (370..372) bands. Multi-instance
/// panels hash the suffixed author_id instead (RISK-004).
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
/// text (200/201/202), cursor (210..274), find-bar (280..283), fold (300..363), nav (370..372), or
/// gutter (400/410../480..) bands. Multi-instance panels hash the suffixed author_id instead (RISK-004).
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
const MATCH_HIGHLIGHT_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(180, 160, 0, 110);
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
    /// Cached highlight spans + the `buffer_version` they were computed for (MT-002 step 3). Recomputed
    /// only when the version changes, so the render path never re-parses every frame.
    highlight_cache: Mutex<Option<(Vec<HighlightSpan>, u64)>>,
    /// Cached measured monospace line height (px), set on the first `show` from
    /// `ui.text_style_height(&TextStyle::Monospace)` (implementation note). `None` until measured.
    line_height_px: Mutex<Option<f32>>,
    /// Per-frame virtualization diagnostics (MT-002 step 4), updated each `show`.
    perf: Mutex<PerfStats>,
    /// The line index range painted on the most recent frame — the exact `row_range`
    /// `egui::ScrollArea::show_rows` passed to the paint closure (AC-007), so tests/agents can assert
    /// exactly which lines are on screen (AC-003) and MT-003+ can position the cursor/gutter/selection
    /// overlay against the real painted window. egui applies NO overscan, so this equals the on-screen
    /// rows, not a padded estimate. `0..0` before the first render.
    last_visible_range: Mutex<std::ops::Range<usize>>,
    /// A one-shot requested vertical scroll offset (px from content top). When set, the next `show`
    /// forces the `ScrollArea` to that offset via `vertical_scroll_offset` and clears the request, so
    /// a caller (a go-to-line action in a later MT, a swarm agent, or a deterministic test) can scroll
    /// the editor to a known position without reaching into egui's persisted scroll state.
    pending_scroll_offset: Mutex<Option<f32>>,
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
    /// once with the SAME `FontId::monospace(MONO_FONT_SIZE)` the glyphs are painted with so a caret at
    /// column `c` lands exactly on column `c`'s glyph (MT-003 positioning requirement). `None` until
    /// measured on the first `show`.
    glyph_width_px: Mutex<Option<f32>>,
    /// The screen-space geometry of the most recent painted row window, captured inside `render_rows`
    /// so the caret/selection overlay and pointer hit-testing share egui's ACTUAL layout (no separate
    /// recompute). `None` before the first render.
    row_geometry: Mutex<Option<RowGeometry>>,
    /// MT-004 in-file find/replace state. `None` when the find bar is closed (no highlights painted —
    /// AC-006); `Some` while it is open. The find bar UI reads + mutates it; `process_find_input`
    /// opens/closes it on Ctrl+F / Ctrl+H / Escape. Behind a `Mutex` for the same `Sync` reason as the
    /// buffer.
    find_state: Mutex<Option<FindState>>,
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
    /// Cached minimap per-row colors + the `(buffer_version, painted_rows, dark_mode)` key they were
    /// computed for. The minimap's only O(spans) pass ([`Minimap::compute_row_colors`]) runs ONLY on a
    /// cache miss (buffer edit, panel resize, or theme flip), so the per-frame minimap render is
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
    /// MT-008 hover tooltip state. `None` when no hover is showing; `Some` while the tooltip is open.
    hover_state: Mutex<Option<HoverState>>,
    /// MT-008 Handshake backend code-nav client (the fallback intelligence source). Reused for
    /// completion + hover + go-to-def + references when no LSP server is attached. Cheap to clone.
    code_nav_client: CodeNavClient,
    /// MT-008 short-lived `lookup_symbols(prefix)` cache (RISK-002 / MC-004 — debounce + cache).
    code_nav_cache: Mutex<CodeNavCache>,
    /// MT-008 LSP client (lazily spawns a language server on first `did_open`). Defaults to
    /// [`LspClient::disabled`] (graceful empty results — AC-004) until a server is configured. Behind a
    /// `Mutex` so the `&self` render/input path can drive it under the `Sync` panel; an `Arc` so the
    /// off-thread completion/hover task can hold it across an await.
    lsp_client: Mutex<Arc<LspClient>>,
    /// MT-008 active workspace id used for the backend code-nav lookups (empty = no workspace bound,
    /// so code-nav requests are skipped — the React `activeWorkspaceId() == null` short-circuit).
    workspace_id: Mutex<String>,
    /// MT-008 instant of the last buffer edit (implementation note 2). The completion trigger only
    /// fires when this is at least [`COMPLETION_DEBOUNCE_MS`] in the past, so fast typing does not flood
    /// the backend (RISK-002). `None` until the first edit.
    last_edit_instant: Mutex<Option<std::time::Instant>>,
    /// MT-008 hover-dwell tracker (implementation note 3): the `(cursor_byte_offset, since)` the cursor
    /// has rested at. A hover request fires once the dwell exceeds [`HOVER_DWELL_MS`] at the same
    /// offset. `None` when the cursor is moving / no dwell is in progress.
    hover_dwell: Mutex<Option<(usize, std::time::Instant)>>,
    /// MT-008 off-thread completion result delivery cell. A spawned `lookup_symbols` task writes the
    /// `(anchor, items)` here; the next `show` drains it into `completion_state` (HBR-QUIET — the egui
    /// thread never blocks on the backend). `Arc<Mutex<..>>` so the spawned task + the UI thread share it.
    completion_result: CompletionResultCell,
    /// MT-008 off-thread hover result delivery cell. A spawned hover task writes the `(anchor, hover)`
    /// here; the next `show` drains it into `hover_state`.
    hover_result: HoverResultCell,
    /// MT-010 off-thread go-to-definition result cell (F12). A spawned `lookup_symbols` task writes the
    /// resolved 0-based definition line here; the next `show` drains it and calls `navigate_to_line`.
    /// Reuses the MT-008 code-nav client + the MT-006 line-navigation path (no new backend surface).
    goto_def_result: GotoDefResultCell,
    /// MT-010 off-thread references result cell (Shift+F12). A spawned `get_references` task writes the
    /// callers/callees here; the next `show` drains it into `last_references` for the observable accessor.
    references_result: ReferencesResultCell,
    /// MT-010 the most recent ShowReferences result (callers + callees), exposed via
    /// [`last_references`](Self::last_references) so tests/agents can observe the backend round-trip even
    /// though the references-panel UI is a follow-on MT (no rendered panel in MT-010 scope).
    last_references: Mutex<Option<CodeSymbolReferencesResponse>>,
    /// MT-008 the LSP `publishDiagnostics` receiver, parked on the panel after it is taken (once) from
    /// the LSP client, so [`drain_lsp_diagnostics`](Self::drain_lsp_diagnostics) can incrementally drain
    /// it each frame and route notifications to the gutter (AC-008). `None` until the first drain takes
    /// the receiver from a configured client.
    lsp_diagnostics_rx:
        Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<PublishedDiagnostics>>>,
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
    completion_request: std::sync::atomic::AtomicBool,

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

    // ── MT-051 line-edit buffer transforms ────────────────────────────────────────────────────────

    /// MT-051 the queued single-undo snapshot `(description, before_text, after_text)` for a just-applied
    /// line transform (ToggleComment / DuplicateLine / MoveLine / DeleteLine / Indent / Dedent / InsertTab).
    /// Each `line_ops` transform snapshots the whole buffer before + after and queues ONE entry here; the
    /// factory render drains it into `interop_adapter::push_code_edit_undo` so a single Ctrl+Z reverts the
    /// whole transform (RISK-003 / AC-007) — the SAME bus boundary every code edit's undo is recorded at
    /// (the MT-035/050 wrap-not-fork pattern; no parallel undo stack). Only the latest is kept (a second
    /// transform before the drain supersedes; the host applies them in order so the newest pair is correct).
    pending_line_op_undo: Mutex<Option<(&'static str, String, String)>>,
    /// MT-051 the operator's `editor.tabSize` (one indent unit = this many spaces when `insert_spaces`).
    /// Sourced from the editor-settings layer via [`set_indent_settings`](Self::set_indent_settings);
    /// defaults to VS Code's 4. Atomic so the `&self` dispatch reads it without locking. Never hardcoded
    /// at a `line_ops` call site (MC-006).
    tab_size: AtomicU64,
    /// MT-051 the operator's `editor.insertSpaces`: when true one indent unit is `tab_size` spaces, when
    /// false it is a literal tab (RISK-006 / MC-006). Defaults to VS Code's true. Atomic for `&self` reads.
    insert_spaces: std::sync::atomic::AtomicBool,

    // ── MT-054 editor chrome: word wrap + bracket match/colorize + indent guides ──────────────────

    /// MT-054 the word-wrap configuration (Alt+Z). `enabled == false` by default (the MT-002 baseline
    /// 1:1 render — RISK-006 / MC-006). The `show` path consumes the Alt+Z shortcut to flip `enabled`,
    /// refreshes `viewport_width_px` each frame from the live editor-area width, and drives BOTH the
    /// `show_rows` row count + scroll math AND the per-row paint from the resulting VisualRow list
    /// (RISK-001 / MC-001 — one source of truth). Behind a `Mutex` for the same `Sync` reason as the
    /// buffer; a swarm agent flips it via the `editor-wrap-toggle` AccessKit node.
    wrap_config: Mutex<WrapConfig>,
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
    command_palette_tx: Mutex<Option<mpsc::Sender<CodeEditorAction>>>,
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
}

/// MT-034 off-thread find-notes result delivery cell: the resolved [`NoteRefsState`] written by a spawned
/// `find_notes_referencing_symbol` task and drained on the next frame into `note_refs_state`. Aliased so
/// the panel field type stays legible (clippy `type_complexity`).
type NoteRefsResultCell = Arc<Mutex<Option<NoteRefsState>>>;

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

/// MT-008 off-thread completion result delivery cell: `(cursor anchor pixel, popup items)` written by
/// a spawned `lookup_symbols` task and drained on the next frame. Aliased so the panel field type stays
/// legible (clippy `type_complexity`).
type CompletionResultCell = Arc<Mutex<Option<(egui::Pos2, Vec<CompletionItem>)>>>;

/// MT-008 off-thread hover result delivery cell: `(cursor anchor pixel, hover state)` written by a
/// spawned hover task and drained on the next frame. Aliased for the same legibility reason.
type HoverResultCell = Arc<Mutex<Option<(egui::Pos2, HoverState)>>>;

/// MT-010 off-thread go-to-definition result cell: the 0-based target buffer line written by a spawned
/// `lookup_symbols` task (F12 / GoToDefinition) and drained on the next frame to `navigate_to_line`.
type GotoDefResultCell = Arc<Mutex<Option<usize>>>;

/// MT-010 off-thread references result cell: the `(callers + callees)` response written by a spawned
/// `get_references` task (Shift+F12 / ShowReferences) and drained on the next frame. There is no
/// references-panel UI in MT-010 scope (that is a follow-on MT — see `lifecycle.blocker` BLOCKER below),
/// so the result is surfaced as an observable accessor + trace line rather than a rendered panel.
type ReferencesResultCell = Arc<Mutex<Option<CodeSymbolReferencesResponse>>>;

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
/// painted_rows, dark_mode)`. Aliased so the `minimap_row_cache` field type stays legible (clippy
/// `type_complexity`).
type MinimapRowCache = (Vec<egui::Color32>, u64, usize, bool);

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
        let mut s = Self { input: one_based.to_string(), parsed: None };
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
#[derive(Clone, Debug, Default)]
pub struct FindState {
    /// The active query (pattern + case/whole-word/regex toggles).
    pub query: FindQuery,
    /// Every match of `query` in the buffer, ascending by byte offset. Recomputed when `query` changes
    /// or after a replace (RISK-003).
    pub matches: Vec<Match>,
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
/// `y = top + (line - first_line) * line_height`. Captured from egui's own row layout so carets align
/// with the glyphs egui actually painted (the MT-002 sans-spacing unit discipline).
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
        let registry = LanguageRegistry::with_bundled_languages();
        let mut highlighter = registry.highlighter_for_extension(extension);
        // Capture the language id from the highlighter (it carries the stable family id), so the fold
        // provider selects the right foldable-node set without re-deriving it every frame (MT-005).
        let language_id = highlighter.as_ref().map(|hl| hl.language_id()).unwrap_or("");
        let buffer = TextBuffer::new(text);
        // Compute the initial fold regions from the first parse tree (when the language is known), so a
        // freshly opened document is foldable on frame 1 (regions start UNfolded — the user/agent folds
        // them). The spans come from the same highlight pass.
        let (spans, fold_set, outline_items) = match highlighter.as_mut() {
            Some(hl) => {
                let spans = hl.highlight(text.as_bytes());
                let (fold_set, outline_items) = match hl.tree() {
                    Some(tree) => {
                        // MC-002: BOTH fold regions and outline symbols derive from the SAME parse tree
                        // via the same TreeCursor pattern — no second parse.
                        let regions = FoldProvider::new().compute(tree, &buffer, language_id);
                        let outline = OutlineProvider::compute(tree, &buffer, language_id);
                        (FoldSet::from_regions(regions), outline)
                    }
                    None => (FoldSet::new(), Vec::new()),
                };
                (spans, fold_set, outline_items)
            }
            None => (Vec::new(), FoldSet::new(), Vec::new()),
        };
        // The outline panel defaults ON only when the document actually has symbols (an empty outline
        // panel adds nothing but takes width — RISK-001). The minimap defaults ON for any document.
        let outline_default_on = !outline_items.is_empty();
        let len_lines = buffer.len_lines();
        // MT-007 breakpoint publish channel: unbounded so `send` never blocks; the receiver is parked
        // until a future DAP client subscribes (RISK-003 non-blocking discard-on-disconnect publish).
        let (breakpoint_sender, breakpoint_receiver) = mpsc::channel::<BreakpointEvent>();
        // MT-049 code-action result channel: the sender is cloned into each spawned `textDocument/codeAction`
        // task; the receiver is parked on the panel until the first pump installs it on the controller.
        let (code_action_tx_init, code_action_rx_init) =
            mpsc::channel::<code_actions::CodeActionResult>();
        Self {
            buffer: Mutex::new(buffer),
            highlighter: Mutex::new(highlighter),
            // Version starts at 1 and the initial spans are cached AT version 1, so the first render
            // is a cache hit (no re-parse) and any later edit bumps to 2+ to invalidate.
            buffer_version: AtomicU64::new(1),
            highlight_cache: Mutex::new(Some((spans, 1))),
            line_height_px: Mutex::new(None),
            perf: Mutex::new(PerfStats {
                frame_lines_rendered: 0,
                buffer_len_lines: len_lines,
                frame_lines_wrapped: 0,
            }),
            last_visible_range: Mutex::new(0..0),
            pending_scroll_offset: Mutex::new(None),
            instance,
            cursor_set: Mutex::new(CursorSet::new()),
            box_drag_start: Mutex::new(None),
            glyph_width_px: Mutex::new(None),
            row_geometry: Mutex::new(None),
            find_state: Mutex::new(None),
            fold_set: Mutex::new(fold_set),
            // Fold regions were computed at buffer version 1 (the same version the highlight cache is
            // filled at), so the first render is a fold cache hit; an edit bumps to 2+ and recomputes.
            fold_version: AtomicU64::new(1),
            language_id,
            extension: extension.to_ascii_lowercase(),
            // The outline was computed at buffer version 1 (same as folds/highlights), so the first
            // access is a cache hit; an edit bumps the version and recomputes from the new tree.
            outline_items: Mutex::new(outline_items),
            outline_version: AtomicU64::new(1),
            show_outline: std::sync::atomic::AtomicBool::new(outline_default_on),
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
            jump_history: Mutex::new(JumpHistory::new()),
            pending_cross_file_jump: Mutex::new(None),
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
            hover_state: Mutex::new(None),
            code_nav_client: CodeNavClient::production(),
            code_nav_cache: Mutex::new(CodeNavCache::new()),
            lsp_client: Mutex::new(Arc::new(LspClient::disabled())),
            workspace_id: Mutex::new(String::new()),
            last_edit_instant: Mutex::new(None),
            hover_dwell: Mutex::new(None),
            completion_result: Arc::new(Mutex::new(None)),
            hover_result: Arc::new(Mutex::new(None)),
            goto_def_result: Arc::new(Mutex::new(None)),
            references_result: Arc::new(Mutex::new(None)),
            last_references: Mutex::new(None),
            lsp_diagnostics_rx: Mutex::new(None),
            runtime: Mutex::new(None),
            completion_request: std::sync::atomic::AtomicBool::new(false),
            // MT-047 signature help: closed until a trigger fires; the delivery cell + request flag start
            // empty; the fallback cache is empty until the first code-nav fallback resolves.
            signature_help_state: Mutex::new(None),
            signature_help_result: Arc::new(Mutex::new(None)),
            signature_help_request: std::sync::atomic::AtomicBool::new(false),
            signature_fallback_cache: Arc::new(Mutex::new(None)),
            // MT-048 rename: starts Idle; the result cell is empty until an off-thread rename resolves.
            rename_state: Mutex::new(RenameState::Idle),
            rename_result: Arc::new(Mutex::new(None)),
            // MT-049 quick fix: an idle controller; the result channel's receiver is installed on the
            // controller lazily on the first pump (one consumer per channel). The cursor-rest debounce +
            // the Ctrl+. arm start empty; the rest threshold is the ~300ms VS Code lightbulb dwell.
            code_action_controller: Mutex::new(CodeActionController::new()),
            code_action_tx: code_action_tx_init,
            code_action_rx: Mutex::new(Some(code_action_rx_init)),
            code_action_rest: Mutex::new(None),
            code_action_rest_threshold: Mutex::new(std::time::Duration::from_millis(
                CODE_ACTION_REST_MS,
            )),
            quick_fix_request: std::sync::atomic::AtomicBool::new(false),
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
            tab_size: AtomicU64::new(4),
            insert_spaces: std::sync::atomic::AtomicBool::new(true),
            // MT-054 word wrap: OFF by default so the first render is the MT-002 1:1 baseline
            // (RISK-006 / MC-006). The viewport width is filled in each frame from the live editor-area
            // width before the wrap layout runs.
            wrap_config: Mutex::new(WrapConfig::default()),
            wrap_row_index: Mutex::new(None),
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
            note_refs_result: Arc::new(Mutex::new(None)),
            find_notes_backend: Mutex::new(Arc::new(FindNotesHttp::production())),
            note_refs_dwell_threshold: Mutex::new(std::time::Duration::from_millis(
                crate::interop::NOTE_REFS_DWELL_MS,
            )),
        }
    }

    /// A cheap snapshot clone of the document buffer (ropey clones share structure O(1)). Returns an
    /// owned [`TextBuffer`] rather than a borrow because the buffer now lives behind a `Mutex` (MT-003:
    /// edits made it interior-mutable). Tests/later MTs read line counts / text through it.
    pub fn buffer(&self) -> TextBuffer {
        self.buffer.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).len()
    }

    /// Replace the whole cursor set with one caret at `byte_offset` (a plain, non-Alt click). Clamped
    /// + char-snapped to the buffer.
    pub fn set_single_cursor(&self, byte_offset: usize) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        self.cursor_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .set_primary(byte_offset, &buffer);
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
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            self.cursor_set
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert_at_all(text, &mut buffer)
        };
        if applied > 0 {
            self.refresh(); // bump version + recompute highlights (RISK-002 invalidation).
        }
        applied
    }

    /// Replace the WHOLE buffer with `text` and re-highlight (MT-035 undo-snapshot restore). The unified
    /// undo scope's `undo_fn` for a code edit captures a [`TextBuffer`] snapshot taken BEFORE the edit
    /// (ropey clones are O(1) — implementation note 1/2) and calls this to restore it on Ctrl+Z. Bumping
    /// the buffer version through [`Self::refresh`] invalidates the stale highlight spans (RISK-002, the
    /// length-changing-undo case the buffer-version hook documents). Cursors are clamped to the new
    /// length so a restored shorter document never leaves an out-of-range caret (panic-free — AC-006
    /// spirit). Returns the new byte length.
    pub fn set_text(&self, text: &str) -> usize {
        let new_len = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            *buffer = TextBuffer::new(text);
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
        new_len
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
        self.refresh_find_matches();
    }

    /// Close the find bar: clears `find_state` so no match highlights paint on the next frame (AC-006).
    pub fn close_find(&self) {
        *self.find_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// True when the find bar is open (a frame would paint match highlights). The render loop and tests
    /// read this; `find_state().is_some()` is the native analog of Monaco's `findWidgetVisible`.
    pub fn is_find_open(&self) -> bool {
        self.find_state.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// A snapshot clone of the current find state (for tests + the overlay). `None` when the bar is
    /// closed.
    pub fn find_state(&self) -> Option<FindState> {
        self.find_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        let target_line = {
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
            state.current().map(|m| m.line)
        };
        if let Some(line) = target_line {
            self.scroll_to_line(line);
        }
    }

    /// Set the query pattern (called by the find input each frame when the text changes) and re-search.
    /// A no-op when the bar is closed.
    pub fn set_find_query(&self, pattern: impl Into<String>) {
        {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_mut() else { return };
            state.query.pattern = pattern.into();
        }
        self.refresh_find_matches();
    }

    /// Set a toggle (case-sensitive / whole-word / regex) and re-search. A no-op when the bar is closed.
    pub fn set_find_toggles(&self, case_sensitive: bool, whole_word: bool, is_regex: bool) {
        {
            let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_mut() else { return };
            state.query.case_sensitive = case_sensitive;
            state.query.whole_word = whole_word;
            state.query.is_regex = is_regex;
        }
        self.refresh_find_matches();
    }

    /// Set the replace text (called by the replace input). A no-op when the bar is closed.
    pub fn set_replace_text(&self, text: impl Into<String>) {
        let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = guard.as_mut() {
            state.replace_text = text.into();
        }
    }

    /// Replace the CURRENT match with the replace text, then re-search (RISK-003: the remaining match
    /// offsets are stale after a buffer edit, so we always re-run search before reusing the list).
    /// Returns `true` when a replacement was applied. The current-match index is preserved (clamped to
    /// the new, smaller match count) so repeated Replace walks through the occurrences.
    pub fn replace_current(&self) -> bool {
        let (target, replacement) = {
            let guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_ref() else { return false };
            match state.current() {
                Some(m) => (m.clone(), state.replace_text.clone()),
                None => return false,
            }
        };
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            FindEngine::replace_one(&mut buffer, &target, &replacement)
        };
        if applied {
            self.refresh(); // re-highlight (RISK-002 invalidation, edit changed the buffer)
            self.refresh_find_matches(); // RISK-003: recompute the now-stale match list
        }
        applied
    }

    /// Replace ALL matches with the replace text, then re-search. Returns the number of replacements
    /// applied. `FindEngine::replace_all` processes in reverse byte order so offsets stay valid within
    /// the batch (RISK-003 / MC-002); we still re-search afterward so the (now-empty-or-changed) match
    /// list is correct.
    pub fn replace_all(&self) -> usize {
        let (matches, replacement) = {
            let guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
            let Some(state) = guard.as_ref() else { return 0 };
            (state.matches.clone(), state.replace_text.clone())
        };
        if matches.is_empty() {
            return 0;
        }
        let applied = {
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            FindEngine::replace_all(&mut buffer, &matches, &replacement)
        };
        if applied > 0 {
            self.refresh();
            self.refresh_find_matches();
        }
        applied
    }

    /// Re-run [`FindEngine::search`] for the current query over the current buffer and store the result
    /// in `find_state`, clamping `current_match` into range and recording the regex compile error
    /// (AC-003). The single place matches are recomputed; called when the query/toggles change and
    /// after any replace (RISK-003). A no-op when the bar is closed.
    fn refresh_find_matches(&self) {
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
        let mut guard = self.find_state.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = guard.as_mut() else { return };
        state.matches = FindEngine::search(&state.query, &buffer);
        state.error = FindEngine::compile_error(&state.query).unwrap_or_default();
        state.last_searched = state.query.pattern.clone();
        state.last_toggles =
            (state.query.case_sensitive, state.query.whole_word, state.query.is_regex);
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
            .and_then(|c| c.as_ref().map(|(spans, _)| spans.clone()))
            .unwrap_or_default()
    }

    /// Re-run highlighting over the current buffer (called after an edit). Bumps `buffer_version` so
    /// the highlight cache is invalidated, then recomputes — this is the path an edit/undo/redo in
    /// MT-003 will call. No-op highlighter -> empty spans. `&self` (interior-mutable) so it composes
    /// with the `Arc`-held render panel.
    pub fn refresh(&self) {
        self.buffer_version.fetch_add(1, Ordering::Relaxed);
        self.ensure_highlight_cache();
    }

    /// Recompute the highlight cache iff it is missing or stale (its stored version != the current
    /// `buffer_version`). Idempotent and cheap on a cache hit (just a version compare). This is the
    /// single place spans are parsed, so the render path is guaranteed not to re-parse on a hit
    /// (MT-002 step 3).
    fn ensure_highlight_cache(&self) {
        let version = self.buffer_version.load(Ordering::Relaxed);
        {
            let cache = self.highlight_cache.lock().unwrap_or_else(|e| e.into_inner());
            if matches!(cache.as_ref(), Some((_, v)) if *v == version) {
                return; // cache hit: no re-parse this frame (MT-002 step 3).
            }
        }
        // Miss: parse once, under the highlighter lock, then store the spans at this version.
        let bytes = self.with_buffer(|b| b.to_bytes());
        let spans = match self.highlighter.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            Some(hl) => hl.highlight(&bytes),
            None => Vec::new(),
        };
        *self.highlight_cache.lock().unwrap_or_else(|e| e.into_inner()) = Some((spans, version));
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
        self.fold_set.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Toggle the fold whose region starts on buffer line `start_line`. Returns `true` when a region
    /// existed on that line (folded state flipped), `false` otherwise. The gutter fold-triangle click
    /// handler (MT-007) and the Ctrl+Shift+[ / Ctrl+Shift+] keymap call this; idempotent in pairs
    /// (AC-006: two toggles return to the original state).
    pub fn toggle_fold(&self, start_line: usize) -> bool {
        self.ensure_fold_regions();
        self.fold_set
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .toggle(start_line)
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
                let region = set
                    .regions
                    .iter()
                    .find(|r| r.start_line == start_line)
                    .map(|r| r.folded);
                if region == Some(folded) {
                    return false; // already in the requested state
                }
                set.toggle(start_line)
            }
            None => false,
        }
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
        *self.outline_items.lock().unwrap_or_else(|e| e.into_inner()) = items;
        self.outline_version.store(version, Ordering::Relaxed);
    }

    /// A snapshot of the current outline symbols (recomputing first if the buffer version moved). For
    /// tests / the outline panel / later MTs (in-file symbol jump — MT-053).
    pub fn outline_items(&self) -> Vec<OutlineItem> {
        self.ensure_outline();
        self.outline_items.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        *self.goto_line_state.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(GotoLineState::for_cursor_line(cursor_line));
    }

    /// Close the go-to-line palette (Escape / after a successful jump / clicking away). A no-op when
    /// already closed.
    pub fn close_goto_line(&self) {
        *self.goto_line_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// True while the go-to-line palette is open.
    pub fn is_goto_line_open(&self) -> bool {
        self.goto_line_state.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// A snapshot of the go-to-line palette state, or `None` when closed.
    pub fn goto_line_state(&self) -> Option<GotoLineState> {
        self.goto_line_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Set the go-to-line input text (the modal's TextEdit pushes the edited value here each frame), and
    /// re-parse it against the live buffer so `parsed` reflects validity. No-op when the palette is
    /// closed.
    pub fn set_goto_line_input(&self, input: impl Into<String>) {
        let len_lines = self.with_buffer(|b| b.len_lines());
        let mut guard = self.goto_line_state.lock().unwrap_or_else(|e| e.into_inner());
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
            let mut guard = self.goto_line_state.lock().unwrap_or_else(|e| e.into_inner());
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
            let mut palette = self.symbol_palette.lock().unwrap_or_else(|e| e.into_inner());
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
            *self.pending_cross_file_jump.lock().unwrap_or_else(|e| e.into_inner()) = None;
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
            *self.pending_cross_file_jump.lock().unwrap_or_else(|e| e.into_inner()) = Some(entry);
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
        *self.diagnostic_markers.lock().unwrap_or_else(|e| e.into_inner()) = markers;
    }

    /// A snapshot of the current diagnostic markers (for tests / the gutter / MT-008).
    pub fn diagnostic_markers(&self) -> Vec<GutterMarker> {
        self.diagnostic_markers.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// A snapshot clone of the breakpoint set (for tests / the gutter / a future DAP client).
    pub fn breakpoint_set(&self) -> BreakpointSet {
        self.breakpoint_set.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// True when buffer `line` carries a breakpoint.
    pub fn is_breakpoint_set(&self, line: usize) -> bool {
        self.breakpoint_set.lock().unwrap_or_else(|e| e.into_inner()).contains(line)
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
        let file_path = self.file_path.lock().unwrap_or_else(|e| e.into_inner()).clone();
        // RISK-003 / MC-003: non-blocking publish; `.ok()` discards the Err when the receiver is gone.
        self.breakpoint_sender
            .send(BreakpointEvent { file_path, line, action })
            .ok();
        action
    }

    /// Take the receive half of the breakpoint channel so a future debug-adapter (DAP) client can
    /// consume published [`BreakpointEvent`]s. Returns the receiver the FIRST time it is called and
    /// `None` afterward (a channel has one consumer). Until a client subscribes, the receiver is parked
    /// inside the panel so publishes are queued rather than dropped; after the receiver is taken and
    /// later dropped, publishes become a benign no-op (RISK-003).
    pub fn subscribe_breakpoints(&self) -> Option<mpsc::Receiver<BreakpointEvent>> {
        self.breakpoint_receiver.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// Set the path of the file this panel edits (carried on every published `BreakpointEvent`).
    pub fn set_file_path(&self, path: impl Into<String>) {
        *self.file_path.lock().unwrap_or_else(|e| e.into_inner()) = path.into();
    }

    /// The path of the file this panel edits (empty for an in-memory buffer).
    pub fn file_path(&self) -> String {
        self.file_path.lock().unwrap_or_else(|e| e.into_inner()).clone()
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

    /// MT-047: the LSP `Position` (0-based line + character) for `byte_offset` in the buffer. The
    /// character is the 0-based COLUMN in the line (the simple byte/char column the editor tracks); a
    /// pedantically-correct UTF-16 code-unit column is a server-tolerated approximation here since the
    /// signature-help request only needs to land inside the call. Never panics (clamps to the line).
    fn lsp_position_at(&self, byte_offset: usize) -> lsp_types::Position {
        let (line, col) = self.with_buffer(|b| byte_to_line_col(byte_offset.min(b.len_bytes()), b));
        lsp_types::Position { line: line as u32, character: col as u32 }
    }

    /// Reset the gutter's per-file state when a new file is loaded into this panel (RISK-004): clears
    /// stale diagnostic markers so a previous file's errors do not appear on the new file, and seeds the
    /// new `file_path` for breakpoint events. (Breakpoints are intentionally NOT cleared here — they are
    /// per-file and a real editor that swaps the panel's document would build a fresh panel; this method
    /// is the seam a same-panel file swap uses, and a swap caller that wants a clean breakpoint slate
    /// can call `clear_breakpoints`.) MT-008's open-file path calls this.
    pub fn load_file(&self, path: impl Into<String>) {
        self.set_file_path(path);
        // RISK-004: clear stale diagnostics from the previous file (no version bump — diagnostics are
        // independent state).
        self.push_diagnostics(Vec::new());
    }

    /// Clear every breakpoint (a full-file reset surface for a same-panel document swap).
    pub fn clear_breakpoints(&self) {
        *self.breakpoint_set.lock().unwrap_or_else(|e| e.into_inner()) = BreakpointSet::new();
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
            format!("{CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX}{line}#{}", self.instance)
        }
    }

    /// The stable AccessKit author_id for the diagnostic marker on buffer `line`, with the instance
    /// suffix (`code_editor_diagnostic_{line}`).
    pub fn diagnostic_author_id(&self, line: usize) -> String {
        if self.instance.is_empty() {
            format!("{CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX}{line}")
        } else {
            format!("{CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX}{line}#{}", self.instance)
        }
    }

    /// The screen rect the gutter strip occupied on the most recent frame, or `None` before the first
    /// render. The basis for the AC-003/AC-005 gutter layout + click tests.
    pub fn last_gutter_rect(&self) -> Option<egui::Rect> {
        *self.last_gutter_rect.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The buffer line of each PAINTED gutter row on the most recent frame, in painted order. The
    /// deterministic basis for the AC-004 (all 10 lines painted) + AC-006 (a folded body line is no
    /// longer painted) gutter tests.
    pub fn gutter_rows_for_test(&self) -> Vec<usize> {
        self.last_gutter_rows.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
        self.completion_request.load(Ordering::Relaxed)
    }

    // ── MT-008 code intelligence API (completion / hover / code-nav / LSP) ─────────────────────────

    /// Bind the active workspace id used for backend code-nav lookups. Empty = no workspace (code-nav
    /// requests short-circuit to empty, the React `activeWorkspaceId() == null` behavior).
    pub fn set_workspace_id(&self, workspace_id: impl Into<String>) {
        *self.workspace_id.lock().unwrap_or_else(|e| e.into_inner()) = workspace_id.into();
    }

    /// The active workspace id (empty when unbound).
    pub fn workspace_id(&self) -> String {
        self.workspace_id.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Replace the LSP client (e.g. install a configured language server, or a mock LSP in a test). The
    /// default is [`LspClient::disabled`] (graceful empty results — AC-004).
    pub fn set_lsp_client(&self, client: Arc<LspClient>) {
        *self.lsp_client.lock().unwrap_or_else(|e| e.into_inner()) = client;
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
        self.runtime.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The primary cursor's head byte offset (the live-loop hover-dwell / completion-prefix anchor).
    fn primary_cursor_offset(&self) -> usize {
        self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).primary().head
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

        let Some(runtime) = self.runtime_handle() else {
            // No runtime injected: clear any armed completion / signature-help request so it does not
            // fire later, and skip the off-thread triggers (the synthetic open_completion/open_hover/
            // open_signature_help test paths and the diagnostics drain above still work without a
            // runtime).
            self.completion_request.store(false, Ordering::Relaxed);
            self.signature_help_request.store(false, Ordering::Relaxed);
            return;
        };

        // HOVER: advance the dwell clock for the live caret offset; on a dwell hit, fetch the hover for
        // the word under the caret (a no-op when the caret is not in a word / no workspace is bound).
        let offset = self.primary_cursor_offset();
        if self.update_hover_dwell(offset) && !self.is_hover_open() {
            let word = self.word_at_primary_cursor();
            if !word.is_empty() {
                self.trigger_hover(&runtime, &word);
            }
        }

        // COMPLETION: fire only when armed this frame (Ctrl+Space / trigger char) — the debounce +
        // 2-char + workspace guards live inside `trigger_completion`.
        let armed = self.completion_request.swap(false, Ordering::Relaxed);
        if armed {
            let word = self.word_at_primary_cursor();
            if !word.is_empty() {
                self.trigger_completion(&runtime, &word);
            }
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
    }

    /// A snapshot of the completion popup state (`None` when no popup is showing). For tests + the
    /// input handler.
    pub fn completion_state(&self) -> Option<CompletionState> {
        self.completion_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// True while the completion popup is showing.
    pub fn is_completion_open(&self) -> bool {
        self.completion_state.lock().unwrap_or_else(|e| e.into_inner()).is_some()
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
        *self.completion_state.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(CompletionState::new(items, anchor));
    }

    /// Close the completion popup (Escape / after accept / no items).
    pub fn close_completion(&self) {
        *self.completion_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Move the completion selection down (ArrowDown). A no-op when closed.
    pub fn completion_select_next(&self) {
        if let Some(state) = self.completion_state.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            state.select_next();
        }
    }

    /// Move the completion selection up (ArrowUp). A no-op when closed.
    pub fn completion_select_prev(&self) {
        if let Some(state) = self.completion_state.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            state.select_prev();
        }
    }

    /// Accept the currently-selected completion item (Enter): insert its `insert_text` at the cursor,
    /// then close the popup. Returns `true` when an item was inserted. The single accept path the Enter
    /// keymap + the popup click both funnel through.
    pub fn accept_completion(&self) -> bool {
        let insert = {
            let guard = self.completion_state.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|s| s.selected().map(|i| i.insert_text.clone()))
        };
        match insert {
            Some(text) => {
                self.insert_text(&text);
                self.close_completion();
                true
            }
            None => false,
        }
    }

    /// Accept the completion item at `index` (a click on a specific row). Inserts + closes.
    pub fn accept_completion_index(&self, index: usize) -> bool {
        let insert = {
            let guard = self.completion_state.lock().unwrap_or_else(|e| e.into_inner());
            guard.as_ref().and_then(|s| s.items.get(index).map(|i| i.insert_text.clone()))
        };
        match insert {
            Some(text) => {
                self.insert_text(&text);
                self.close_completion();
                true
            }
            None => false,
        }
    }

    /// A snapshot of the hover tooltip state (`None` when no tooltip is showing).
    pub fn hover_state(&self) -> Option<HoverState> {
        self.hover_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// True while the hover tooltip is showing.
    pub fn is_hover_open(&self) -> bool {
        self.hover_state.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// Open the hover tooltip with `state` (the deterministic path the dwell trigger + tests use).
    pub fn open_hover(&self, state: HoverState) {
        *self.hover_state.lock().unwrap_or_else(|e| e.into_inner()) = Some(state);
    }

    /// Close the hover tooltip (cursor moved / Escape / after go-to-def).
    pub fn close_hover(&self) {
        *self.hover_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    // ── MT-047 signature help (parameter hints) — public API + triggers ───────────────────────────

    /// A snapshot of the signature-help popup state (`None` when no popup is showing). The deterministic
    /// observation point for the kittest/unit proofs.
    pub fn signature_help_state(&self) -> Option<SignatureHelpState> {
        self.signature_help_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// True while the signature-help popup is showing.
    pub fn is_signature_help_open(&self) -> bool {
        self.signature_help_state.lock().unwrap_or_else(|e| e.into_inner()).is_some()
    }

    /// Open the signature-help popup with `state` (the deterministic path the trigger spawn delivers
    /// into + the tests drive directly).
    pub fn open_signature_help(&self, state: SignatureHelpState) {
        *self.signature_help_state.lock().unwrap_or_else(|e| e.into_inner()) = Some(state);
    }

    /// Close the signature-help popup (`)`/Escape/cursor leaving the call/focus loss) and clear the
    /// fallback cache so a fresh call site re-resolves (RISK-002).
    pub fn close_signature_help(&self) {
        *self.signature_help_state.lock().unwrap_or_else(|e| e.into_inner()) = None;
        *self.signature_fallback_cache.lock().unwrap_or_else(|e| e.into_inner()) = None;
    }

    /// Cycle the active overload to the NEXT signature (Down arrow while the popup is open). No-op when
    /// the popup is closed / there is only one signature.
    pub fn signature_help_next(&self) {
        if let Some(state) = self.signature_help_state.lock().unwrap_or_else(|e| e.into_inner()).as_mut()
        {
            state.select_next_signature();
        }
    }

    /// Cycle the active overload to the PREVIOUS signature (Up arrow while the popup is open).
    pub fn signature_help_prev(&self) {
        if let Some(state) = self.signature_help_state.lock().unwrap_or_else(|e| e.into_inner()).as_mut()
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
        let prefix = self.with_buffer(|b| b.byte_slice_to_string(0..cursor_byte.min(b.len_bytes())));
        find_enclosing_open_paren(&prefix)
    }

    /// The identifier token immediately to the LEFT of `open_paren_byte` (the call target), or an empty
    /// string when there is none. Used to resolve the fallback signature via the code-nav client.
    fn call_target_identifier(&self, open_paren_byte: usize) -> String {
        let prefix = self
            .with_buffer(|b| b.byte_slice_to_string(0..open_paren_byte.min(b.len_bytes())));
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
        let code_nav = self.code_nav_client.clone();
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
        let glyph_width = (*self.glyph_width_px.lock().unwrap_or_else(|e| e.into_inner()))?;
        let (line, col) = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let head = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).primary().head;
            byte_to_line_col(head, &buffer)
        };
        let pos = self.screen_pos_for_line_col(line, col, glyph_width)?;
        // Anchor a touch below the caret so the popup does not cover the current line.
        Some(pos + egui::vec2(0.0, 14.0))
    }

    // ── MT-048 Rename Symbol (F2) — public API + triggers ─────────────────────────────────────────

    /// A snapshot of the rename state (the deterministic observation point for the kittest/unit proofs).
    pub fn rename_state(&self) -> RenameState {
        self.rename_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
                RenameState::Editing { original, draft, ident_range, .. } => {
                    (original.clone(), draft.clone(), ident_range.clone())
                }
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
                }
                *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = RenameState::Idle;
                Some(report)
            }
            Err(e) => {
                *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) =
                    RenameState::Error { message: e.to_string() };
                None
            }
        }
    }

    /// MT-048: drain a delivered off-thread rename result into the rename state (called each frame). An
    /// `Ok(preview)` becomes `Previewing` (or stays Idle on an empty no-op preview with a trace); an
    /// `Err(message)` becomes `Error`. A no-op when no result is pending.
    fn drain_rename_result(&self) {
        let pending = self.rename_result.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(result) = pending {
            match result {
                Ok(preview) if preview.is_empty() => {
                    // No changes (the server declined / nothing to rename): return to Idle silently.
                    *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) = RenameState::Idle;
                    tracing::debug!("code editor: rename produced no changes");
                }
                Ok(preview) => {
                    *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) =
                        RenameState::Previewing { workspace_edit: preview };
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
        self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner()).is_menu_open()
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
        let guard = self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner());
        match guard.state() {
            Some(s) => s.actions.iter().map(|a| a.title.clone()).collect(),
            None => Vec::new(),
        }
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
        self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner()).close_menu();
    }

    /// Clear the quick-fix controller to idle (no lightbulb, no menu).
    pub fn clear_quickfix(&self) {
        self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner()).clear();
    }

    /// Set the quick-fix cursor-rest debounce threshold (a kittest sets it to ZERO so the rest crossing
    /// fires on the first settled frame — the same deterministic-dwell hook the MT-034 note-refs path uses).
    pub fn set_quickfix_rest_threshold(&self, threshold: std::time::Duration) {
        *self.code_action_rest_threshold.lock().unwrap_or_else(|e| e.into_inner()) = threshold;
    }

    /// Whether the Ctrl+. quick-fix request is currently armed (not yet consumed by the pump). The
    /// live-path test reads it to prove the pump CONSUMED the arm in the same frame.
    pub fn quick_fix_request_armed_for_test(&self) -> bool {
        self.quick_fix_request.load(Ordering::Relaxed)
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
        let markers = self.diagnostic_markers.lock().unwrap_or_else(|e| e.into_inner());
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
            start: lsp_types::Position { line: line as u32, character: start_char },
            end: lsp_types::Position { line: line as u32, character: end_char },
        }
    }

    /// MT-049: spawn the off-thread `textDocument/codeAction` request for `line` and deliver the normalized
    /// actions over the result channel (drained next frame by [`pump_code_actions`](Self::pump_code_actions)).
    /// `open_menu` opens the menu when the result lands (the Ctrl+. / lightbulb / context-menu path) vs only
    /// lighting the bulb (the passive cursor-rest path). The egui thread never blocks (HBR-QUIET): the LSP
    /// request runs on the injected runtime. When no LSP is attached the request returns an empty action
    /// list (graceful — AC-006), which still lands so the Ctrl+. degraded menu can show "No quick fixes".
    pub fn trigger_quick_fix(&self, runtime: &tokio::runtime::Handle, line: usize, open_menu: bool) {
        let version = self.buffer_version.load(Ordering::Relaxed);
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
        if let Some(rx) = self.code_action_rx.lock().unwrap_or_else(|e| e.into_inner()).take() {
            self.code_action_controller
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .set_result_receiver(rx);
        }
        // (2) Drain any delivered result into the controller state (lights the bulb / opens the menu).
        self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner()).poll_results();

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
        let threshold = *self.code_action_rest_threshold.lock().unwrap_or_else(|e| e.into_inner());
        let now = std::time::Instant::now();
        let mut rest = self.code_action_rest.lock().unwrap_or_else(|e| e.into_inner());
        let on_diagnostic_line = self.line_has_diagnostic(cursor_line);
        if !on_diagnostic_line {
            // Off a diagnostic line: reset the dwell and clear any stale actions for a now-irrelevant line.
            *rest = None;
            let mut controller = self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner());
            if controller.active_line().map(|l| l != cursor_line).unwrap_or(false)
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
                let controller =
                    self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner());
                controller.request_in_flight()
                    || controller.active_line() == Some(cursor_line) && controller.has_actions_on_line(cursor_line)
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
            let mut controller =
                self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner());
            let mut buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner()).clone();
            let mut cursors = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).clone();
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
                self.set_text(&new_text);
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
        self.lsp_uri()
            .or_else(|| {
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
        self.last_format_toast.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
            *self.last_format_toast.lock().unwrap_or_else(|e| e.into_inner()) =
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
            *self.last_format_toast.lock().unwrap_or_else(|e| e.into_inner()) =
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
            self.format_selection_request.store(false, Ordering::Relaxed);
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
                    Err(e) => (None, FormatOutcome::LspError(format!("Formatting failed: {e}"))),
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
                    Err(e) => (None, FormatOutcome::LspError(format!("Formatting failed: {e}"))),
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
        let pending = self.format_result.lock().unwrap_or_else(|e| e.into_inner()).take();
        let Some((before, formatted, outcome)) = pending else { return };
        match &outcome {
            FormatOutcome::Applied { edit_count } => {
                if let Some(after) = formatted {
                    if after != before {
                        // Install the formatted text (re-clamps the cursor — RISK-006) and record ONE undo
                        // entry (before -> after) so a single Ctrl+Z reverts the WHOLE format (AC-001).
                        self.set_text(&after);
                        self.record_format_undo(&before, &after);
                        tracing::debug!("code editor: formatted document, {edit_count} edits (single undo)");
                    }
                }
            }
            FormatOutcome::NoChange => {
                tracing::debug!("code editor: format produced no changes (already formatted)");
            }
            FormatOutcome::NoFormatter => {
                *self.last_format_toast.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(formatting::NO_FORMATTER_TOOLTIP.to_owned());
            }
            FormatOutcome::LspError(msg) => {
                // A non-blocking toast (NOT a frame-blocking dialog — AC-006 / MC-006). Surfaced + logged.
                tracing::warn!("code editor: format failed: {msg}");
                *self.last_format_toast.lock().unwrap_or_else(|e| e.into_inner()) = Some(msg.clone());
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
        *self.pending_format_undo.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((before.to_owned(), after.to_owned()));
    }

    /// MT-050: take the queued format undo snapshot (before, after) the factory render pushes onto the
    /// shared unified-undo bus. `None` when no format applied since the last drain. The factory drains this
    /// each frame so the single undo entry is recorded at the SAME boundary every code edit's undo is.
    pub fn take_pending_format_undo(&self) -> Option<(String, String)> {
        self.pending_format_undo.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    // ── MT-051 line-edit buffer transforms — settings + dispatch + single-undo ─────────────────────────

    /// MT-051: set the operator's indent settings (`editor.tabSize` + `editor.insertSpaces`) so the
    /// line-edit transforms use them instead of a hardcoded 4 (MC-006 — RISK-006). The host plumbs these
    /// from the editor-settings layer; `tab_size` is clamped to >= 1 (a 0-width indent unit is invalid).
    pub fn set_indent_settings(&self, tab_size: usize, insert_spaces: bool) {
        self.tab_size.store(tab_size.max(1) as u64, Ordering::Relaxed);
        self.insert_spaces.store(insert_spaces, Ordering::Relaxed);
    }

    /// MT-051: the current indent settings `(tab_size, insert_spaces)` (for tests / the host).
    pub fn indent_settings(&self) -> (usize, bool) {
        (
            self.tab_size.load(Ordering::Relaxed) as usize,
            self.insert_spaces.load(Ordering::Relaxed),
        )
    }

    // ── MT-054 word wrap (Alt+Z) — toggle + state ─────────────────────────────────────────────────

    /// MT-054: the current word-wrap configuration (for tests / the host / the AccessKit node value).
    pub fn wrap_config(&self) -> WrapConfig {
        *self.wrap_config.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// MT-054: whether word wrap is currently enabled (Alt+Z toggles it). Persisted on the panel state.
    pub fn is_wrap_enabled(&self) -> bool {
        self.wrap_config.lock().unwrap_or_else(|e| e.into_inner()).enabled
    }

    /// MT-054: flip word wrap on/off and return the NEW enabled state. The single mutation point both
    /// the Alt+Z shortcut and the `editor-wrap-toggle` AccessKit node route through, so the toggle is
    /// deterministic for a swarm agent and persisted on the panel state (AC-005). Render/decoration only
    /// — NO buffer mutation (AC-007).
    pub fn toggle_wrap(&self) -> bool {
        let mut cfg = self.wrap_config.lock().unwrap_or_else(|e| e.into_inner());
        cfg.enabled = !cfg.enabled;
        cfg.enabled
    }

    /// MT-054: explicitly set the wrap-enabled state (host / settings / a test). Persisted on the panel.
    pub fn set_wrap_enabled(&self, enabled: bool) {
        self.wrap_config.lock().unwrap_or_else(|e| e.into_inner()).enabled = enabled;
    }

    /// MT-054: set a fixed wrap COLUMN (`wordWrapColumn`), or `None` to wrap at the viewport edge
    /// (`wordWrap: on`). The host plumbs this from the editor-settings layer; tests use it to force a
    /// deterministic wrap width without a real viewport.
    pub fn set_wrap_column(&self, wrap_column: Option<usize>) {
        self.wrap_config.lock().unwrap_or_else(|e| e.into_inner()).wrap_column = wrap_column;
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
        *self.pending_line_op_undo.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((description, before, after));
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
        let fresh = LanguageRegistry::with_bundled_languages().highlighter_for_extension(&self.extension);
        *self.highlighter.lock().unwrap_or_else(|e| e.into_inner()) = fresh;
    }

    /// MT-051: take the queued line-transform undo snapshot `(description, before, after)` the factory
    /// render pushes onto the shared unified-undo bus as ONE entry. `None` when no transform applied since
    /// the last drain.
    pub fn take_pending_line_op_undo(&self) -> Option<(&'static str, String, String)> {
        self.pending_line_op_undo.lock().unwrap_or_else(|e| e.into_inner()).take()
    }

    /// MT-049: the screen position to anchor the quick-fix lightbulb / menu for `line` — the start of the
    /// line in the gutter strip's lightbulb column (next to the MT-007 diagnostic glyphs). `None` before
    /// the first frame / when the line is off-screen.
    fn quickfix_line_screen_pos(&self, line: usize) -> Option<egui::Pos2> {
        let glyph_width = (*self.glyph_width_px.lock().unwrap_or_else(|e| e.into_inner())).unwrap_or(8.0);
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
        let markers: Vec<GutterMarker> =
            symbols.iter().filter_map(staleness_marker_for).collect();
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
        let receiver = {
            // Take the receiver from the client the first time; it is parked on the panel afterward.
            let client = self.lsp_client.lock().unwrap_or_else(|e| e.into_inner());
            client.take_diagnostics_receiver()
        };
        // The receiver lives on the panel between frames so we drain it incrementally. Store it here.
        if let Some(rx) = receiver {
            *self.lsp_diagnostics_rx.lock().unwrap_or_else(|e| e.into_inner()) = Some(rx);
        }
        let mut latest: Option<PublishedDiagnostics> = None;
        if let Some(rx) = self.lsp_diagnostics_rx.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            // Drain to the most recent notification for this document (LSP replaces the whole set).
            while let Ok(published) = rx.try_recv() {
                latest = Some(published);
            }
        }
        let published = latest?;
        let markers = lsp_diagnostics_to_markers(&published);
        let count = markers.len();
        self.push_diagnostics(markers);
        Some(count)
    }

    /// Mark a buffer edit happened now (the completion-debounce clock — implementation note 2).
    pub fn mark_edit_now(&self) {
        *self.last_edit_instant.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(std::time::Instant::now());
    }

    /// True when the completion debounce window ([`COMPLETION_DEBOUNCE_MS`]) has elapsed since the last
    /// edit (or no edit has happened) — i.e. it is safe to fire a completion request (RISK-002).
    pub fn completion_debounce_elapsed(&self) -> bool {
        match *self.last_edit_instant.lock().unwrap_or_else(|e| e.into_inner()) {
            Some(at) => at.elapsed() >= std::time::Duration::from_millis(COMPLETION_DEBOUNCE_MS),
            None => true,
        }
    }

    /// Spawn an off-thread backend code-nav completion request for `prefix` and deliver the popup items
    /// into the panel's completion-result cell (drained next frame). Caches the lookup (RISK-002 / MC-004).
    /// A no-op when no workspace is bound / the prefix is too short (the React 2-char guard) / the
    /// debounce window has not elapsed. `runtime` is the app's tokio handle (the editor passes it in;
    /// the egui thread never blocks — HBR-QUIET).
    pub fn trigger_completion(&self, runtime: &tokio::runtime::Handle, prefix: &str) {
        let workspace_id = self.workspace_id();
        if workspace_id.is_empty() || prefix.len() < 2 || !self.completion_debounce_elapsed() {
            return;
        }
        let anchor = self
            .cursor_screen_pos()
            .unwrap_or_else(|| egui::pos2(40.0, 40.0));
        // Cache hit: deliver immediately (no spawn).
        if let Some(cached) = self.code_nav_cache.lock().unwrap_or_else(|e| e.into_inner()).get(prefix)
        {
            let items = Self::completions_from_symbols(&cached);
            *self.completion_result.lock().unwrap_or_else(|e| e.into_inner()) = Some((anchor, items));
            return;
        }
        let client = self.code_nav_client.clone();
        let cell = Arc::clone(&self.completion_result);
        let prefix_owned = prefix.to_owned();
        runtime.spawn(async move {
            let symbols = client
                .lookup_symbols(&workspace_id, &prefix_owned, SYMBOL_LOOKUP_LIMIT)
                .await
                .unwrap_or_default(); // graceful empty on backend error (AC-004 analog).
            let items = symbols.iter().map(CompletionItem::from_symbol).collect();
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((anchor, items));
            }
        });
    }

    /// Update the hover-dwell tracker for the current cursor byte offset and return `true` once the
    /// cursor has rested at the SAME offset for at least [`HOVER_DWELL_MS`] (implementation note 3). A
    /// cursor move resets the dwell. The editor calls this each frame with the live cursor offset; on a
    /// `true` it calls [`trigger_hover`](Self::trigger_hover) to fetch the hover.
    pub fn update_hover_dwell(&self, cursor_byte_offset: usize) -> bool {
        let mut guard = self.hover_dwell.lock().unwrap_or_else(|e| e.into_inner());
        match guard.as_ref() {
            Some((offset, since)) if *offset == cursor_byte_offset => {
                since.elapsed() >= std::time::Duration::from_millis(HOVER_DWELL_MS)
            }
            _ => {
                // New offset (or first dwell): restart the dwell clock.
                *guard = Some((cursor_byte_offset, std::time::Instant::now()));
                false
            }
        }
    }

    /// Spawn an off-thread backend code-nav hover request for the identifier `word` and deliver the
    /// rendered hover into the panel's hover-result cell (drained next frame). A no-op when no workspace
    /// is bound / `word` is empty. The hover content is the same data the React `CodeSymbolPanel` shows:
    /// the symbol heading + kind + key + staleness + (when available) the file-lens doc. `runtime` is the
    /// app's tokio handle (the egui thread never blocks — HBR-QUIET).
    pub fn trigger_hover(&self, runtime: &tokio::runtime::Handle, word: &str) {
        let workspace_id = self.workspace_id();
        if workspace_id.is_empty() || word.trim().is_empty() {
            return;
        }
        let anchor = self
            .cursor_screen_pos()
            .unwrap_or_else(|| egui::pos2(40.0, 40.0));
        let client = self.code_nav_client.clone();
        let cell = Arc::clone(&self.hover_result);
        let word_owned = word.to_owned();
        runtime.spawn(async move {
            // Look up the first symbol matching the word (the React `lookupFirstSymbol`).
            let symbols = client
                .lookup_symbols(&workspace_id, &word_owned, 5)
                .await
                .unwrap_or_default();
            let Some(symbol) = symbols.into_iter().next() else {
                return; // no symbol -> no hover (graceful).
            };
            let definition_line = symbol
                .definition
                .as_ref()
                .and_then(|d| d.line_start)
                .filter(|l| *l >= 1)
                .map(|l| (l - 1) as usize);
            let markdown = super::code_nav::markdown_for_symbol(&symbol, None);
            let hover = HoverState {
                markdown,
                display_name: symbol.display_name.clone(),
                anchor,
                definition_line,
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some((anchor, hover));
            }
        });
    }

    /// The screen position of the CENTER of the gutter row that paints buffer `line` on the most recent
    /// frame, or `None` if that line was not painted (off-screen) / no frame has rendered. The
    /// deterministic basis for the AC-005 gutter-click test (compute the exact pixel to click for a
    /// known line). Targets the breakpoint sub-column (left of the gutter) so the click lands on the
    /// breakpoint area, not the line-number or diagnostic column.
    pub fn gutter_breakpoint_pos_for_line(&self, line: usize) -> Option<egui::Pos2> {
        let rows = self.last_gutter_rows.lock().unwrap_or_else(|e| e.into_inner());
        let geometry = (*self.last_gutter_geometry.lock().unwrap_or_else(|e| e.into_inner()))?;
        let rect = (*self.last_gutter_rect.lock().unwrap_or_else(|e| e.into_inner()))?;
        let row_idx = rows.iter().position(|&l| l == line)?;
        let y = geometry.origin.y + row_idx as f32 * geometry.line_height + geometry.line_height * 0.5;
        // Click in the breakpoint sub-column (a little right of the strip's left edge).
        let x = rect.left() + 12.0;
        Some(egui::pos2(x, y))
    }

    /// The screen position of the CENTER of the FOLD sub-column for buffer `line` on the most recent
    /// frame (the fold triangle is left-of-number; this returns its center x), or `None` if the line was
    /// not painted. The basis for the AC-006 gutter fold-click test.
    pub fn gutter_fold_pos_for_line(&self, line: usize) -> Option<egui::Pos2> {
        let rows = self.last_gutter_rows.lock().unwrap_or_else(|e| e.into_inner());
        let geometry = (*self.last_gutter_geometry.lock().unwrap_or_else(|e| e.into_inner()))?;
        let rect = (*self.last_gutter_rect.lock().unwrap_or_else(|e| e.into_inner()))?;
        let config = *self.gutter_config.lock().unwrap_or_else(|e| e.into_inner());
        let row_idx = rows.iter().position(|&l| l == line)?;
        let y = geometry.origin.y + row_idx as f32 * geometry.line_height + geometry.line_height * 0.5;
        // The fold column sits after the breakpoint column. Mirror `gutter::Gutter::render`'s anchors.
        let breakpoint_w = if config.show_breakpoints { 16.0 } else { 0.0 };
        let x = rect.left() + 4.0 + breakpoint_w + 7.0; // center of the 14px fold column
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
        *self.last_minimap_rect.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The screen rect the outline panel occupied on the most recent frame, or `None` before the first
    /// render / while it is hidden. The basis for the AC-003 three-panel layout test (left placement +
    /// width vs the minimap).
    pub fn last_outline_rect(&self) -> Option<egui::Rect> {
        *self.last_outline_rect.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// The cached minimap per-row colors for this frame, recomputing the O(spans) pass ONLY on a cache
    /// miss — a buffer edit (`version` moved), a panel resize (`painted_rows` changed), or a theme flip
    /// (`dark_mode` changed). On a hit (the common per-frame case) this is a cheap key compare + clone of
    /// the small `Vec<Color32>` (at most a few hundred rows), so the minimap render stays O(painted_rows)
    /// instead of O(spans) — the MT-002 frame-budget protection on a 100k-line file.
    fn minimap_row_colors(
        &self,
        painted_rows: usize,
        ratio: usize,
        dark_mode: bool,
        version: u64,
    ) -> Vec<egui::Color32> {
        let key = (version, painted_rows, dark_mode);
        {
            let cache = self.minimap_row_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((colors, v, rows, dm)) = cache.as_ref() {
                if (*v, *rows, *dm) == key && colors.len() == painted_rows {
                    return colors.clone(); // cache hit: no span fetch / no O(spans) re-walk this frame.
                }
            }
        }
        // Miss (edit / resize / theme flip): fetch the cached highlight spans (no extra parse — the
        // highlight cache is already current) and run the single O(spans) color pass, then cache it.
        let colors = {
            let span_cache = self.highlight_cache.lock().unwrap_or_else(|e| e.into_inner());
            let empty: Vec<HighlightSpan> = Vec::new();
            let spans = span_cache.as_ref().map(|(s, _)| s).unwrap_or(&empty);
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            Minimap::compute_row_colors(&buffer, spans, painted_rows, ratio, dark_mode)
        };
        *self.minimap_row_cache.lock().unwrap_or_else(|e| e.into_inner()) =
            Some((colors.clone(), version, painted_rows, dark_mode));
        colors
    }

    /// The per-frame virtualization diagnostics from the most recent `show` (MT-002 step 4). Before
    /// the first render `frame_lines_rendered` is 0; `buffer_len_lines` is always the document size.
    pub fn perf_stats(&self) -> PerfStats {
        *self.perf.lock().unwrap_or_else(|e| e.into_inner())
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

    /// Request that the next `show` scrolls the viewport to `offset_px` (pixels from the content top).
    /// One-shot: the request is consumed (and cleared) on the next frame so the user can scroll freely
    /// afterward. The seam later MTs' go-to-line / scroll-to-symbol actions build on.
    pub fn scroll_to_offset_px(&self, offset_px: f32) {
        *self.pending_scroll_offset.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(offset_px.max(0.0));
    }

    /// Request that the next `show` scrolls so `line` is at the top of the viewport, using the cached
    /// measured line height (or the document is rendered at least once so the height is known). If the
    /// line height has not been measured yet (no frame rendered), the request still stores a best-effort
    /// offset that is corrected on the following frame once the height is known.
    pub fn scroll_to_line(&self, line: usize) {
        let lh = self
            .line_height_px
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(0.0);
        // 0.0 before the first measure -> offset 0; the test/render measures first, then scrolls.
        self.scroll_to_offset_px(line as f32 * lh);
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
        let syntax = syntax_tokens_for(ui.visuals());
        let container_id = self.container_id();
        let container_author = self.container_author_id();
        let scroll_author = self.scroll_author_id();
        let scroll_id = self.scroll_id();
        let text_author = self.text_author_id();
        let text_id = self.text_id();

        // MT-054: consume the Alt+Z word-wrap shortcut BEFORE the keymap dispatch / live-typing loop read
        // input (RISK-005 / MC-005). `consume_shortcut` removes the matching key event from the queue, so
        // neither `process_keymap` nor `process_cursor_input`'s Event::Text path ever sees the 'z' — the
        // toggle flips wrap WITHOUT inserting a literal 'z' into the buffer. Skipped while a rename input
        // owns the keyboard (the same focus-precedence guard `process_keymap` uses) so Alt+Z does not
        // fight the rename surface. The toggle is the SINGLE `toggle_wrap` mutation point the AccessKit
        // node also routes through (AC-005), and it is render-only (no buffer mutation — AC-007).
        if matches!(*self.rename_state.lock().unwrap_or_else(|e| e.into_inner()), RenameState::Idle) {
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
            if ui
                .selectable_label(show_outline, "\u{2261} Outline")
                .on_hover_text("Toggle the outline panel")
                .clicked()
            {
                self.toggle_outline();
            }
            if ui
                .selectable_label(show_minimap, "\u{25A4} Minimap")
                .on_hover_text("Toggle the minimap")
                .clicked()
            {
                self.toggle_minimap();
            }
            // MT-034: toggle the "Notes referencing this symbol" panel (the code->notes cross-ref
            // surface). When shown, dwelling on a symbol loads the notes that mention it (RISK-001 —
            // hideable so the center editor keeps a usable width).
            if ui
                .selectable_label(self.is_note_refs_shown(), "\u{1F4DD} Note refs")
                .on_hover_text("Toggle the panel listing notes that reference the focused code symbol")
                .clicked()
            {
                self.toggle_note_refs();
            }
            // MT-054: the word-wrap toggle (Alt+Z). A visible selectable label that reflects + flips the
            // persisted WrapConfig.enabled through the SAME `toggle_wrap` mutation point Alt+Z and the
            // AccessKit node route through (AC-005). The AccessKit node itself (Role::Button, Toggled
            // property, author_id `editor-wrap-toggle`) is emitted inside the container scope below so it
            // is a container descendant a swarm agent can flip by id.
            if ui
                .selectable_label(self.is_wrap_enabled(), "\u{21B5} Wrap")
                .on_hover_text("Toggle word wrap (Alt+Z)")
                .clicked()
            {
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
            if ui
                .button("\u{2328} Keybindings")
                .on_hover_text(format!("Configure editor keybindings: {keymap_path_label}"))
                .clicked()
            {
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
            *self.last_outline_rect.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(resp.response.rect);
        } else {
            *self.last_outline_rect.lock().unwrap_or_else(|e| e.into_inner()) = None;
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
            *self.last_minimap_rect.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }

        // OUTER container scope (the CENTER editor area). egui gives every child `Ui` its own AccessKit
        // node keyed by the `Ui`'s id and nests it under the parent `Ui`'s node. We emit the CONTAINER
        // node onto THIS scope's own `Ui` id, render the scroll-area in a nested scope inside it, and
        // render the text content nested inside that — so the live tree is container -> scroll-area ->
        // text (AC-004 + AC-005 ancestry). The fixed `container_id` is only the `id_salt` that keeps the
        // scope's id stable across frames.
        ui.scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
            let container_node_id = ui.unique_id();

            // Paint the editor background from the theme (no hardcoded hex).
            let bg = syntax.background;
            let full_rect = ui.available_rect_before_wrap();
            if ui.is_rect_visible(full_rect) {
                ui.painter().rect_filled(full_rect, 0.0, bg);
            }

            // MT-048: the Editor Body Context Menu code-pane 'Rename Symbol' entry. A secondary-click
            // anywhere over the editor body opens the menu; choosing 'Rename Symbol' runs the SAME
            // `begin_rename` path as F2. Built through egui's `response.context_menu` (the same surface the
            // WP-011 context_menu_surfaces uses for simple panes) with the exact contract author_id +
            // Role::MenuItem AccessKit node so a swarm agent can trigger it by id (AC-005 / HBR-SWARM).
            self.render_editor_context_menu(ui, full_rect);

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
            *self.last_gutter_rect.lock().unwrap_or_else(|e| e.into_inner()) = Some(gutter_rect);

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
                scroll_area.show_rows(
                    ui,
                    line_height,
                    scroll_row_count,
                    |ui, row_range| {
                        // Record egui's actual painted window before painting.
                        painted_range = row_range.clone();
                        if wrap_enabled {
                            // MT-054 PERF CAP: materialize ONLY the painted visual-row window's logical
                            // lines (O(window)), translated from the cached index — not the whole doc.
                            let (window_rows, window_start, lines_touched) =
                                self.wrap_rows_for_window(row_range.clone(), &wrap_cfg, glyph_width);
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
                    },
                );

                // MT-003: process multi-cursor input AFTER the rows painted this frame, so the captured
                // row geometry is available to map a pointer position (Alt+Click / box drag) to a
                // (line, col) byte offset. Reads egui input events from this scroll scope's `ui`.
                self.process_cursor_input(ui, line_height, glyph_width, total_lines);

                // Store egui's actual painted row range as BOTH the perf "lines painted this frame"
                // count and the `last_visible_range` overlay seam (AC-007). The painted range is the
                // ground truth MT-003+ reads to position the cursor/gutter/selection overlay, so the
                // diagnostics must equal it exactly — not the overscan-padded calculator estimate.
                let stats = PerfStats {
                    frame_lines_rendered: painted_range.len(),
                    buffer_len_lines: total_lines,
                    frame_lines_wrapped,
                };
                *self.perf.lock().unwrap_or_else(|e| e.into_inner()) = stats;
                *self.last_visible_range.lock().unwrap_or_else(|e| e.into_inner()) =
                    painted_range.clone();

                // Emit the ScrollView node onto THIS scroll scope's Ui id (AC-004). It is a child of
                // the container scope and the parent of the text scope.
                let author = scroll_author.clone();
                ui.ctx().accesskit_node_builder(scroll_node_id, move |node| {
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
            self.render_gutter(ui, gutter_rect, gutter_glyph_width, &gutter_cfg);

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
            ui.ctx().accesskit_node_builder(container_node_id, move |node| {
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
        if let Some((anchor, items)) =
            self.completion_result.lock().unwrap_or_else(|e| e.into_inner()).take()
        {
            if items.is_empty() {
                self.close_completion();
            } else {
                *self.completion_state.lock().unwrap_or_else(|e| e.into_inner()) =
                    Some(CompletionState::new(items, anchor));
            }
        }
        // Drain a delivered hover result.
        if let Some((_anchor, hover)) =
            self.hover_result.lock().unwrap_or_else(|e| e.into_inner()).take()
        {
            self.open_hover(hover);
        }
        // Drain a delivered go-to-definition target (F12): jump the caret + scroll to the def line.
        if let Some(line) = self.goto_def_result.lock().unwrap_or_else(|e| e.into_inner()).take() {
            // MT-052 jump-history record site #2 (goto-definition): record the PRE-jump caret location so
            // Navigate Back returns to the call site, BEFORE the caret jumps to the definition line.
            self.record_jump_origin();
            self.navigate_to_line(line);
        }
        // Drain a delivered references result (Shift+F12): park it for the observable accessor. No
        // rendered references panel in MT-010 scope (follow-on MT — see handoff BLOCKER).
        if let Some(refs) = self.references_result.lock().unwrap_or_else(|e| e.into_inner()).take() {
            tracing::debug!(
                total = refs.total(),
                callers = refs.callers.len(),
                callees = refs.callees.len(),
                "code editor: ShowReferences result delivered"
            );
            *self.last_references.lock().unwrap_or_else(|e| e.into_inner()) = Some(refs);
        }

        // Render the completion popup (a no-op when closed). The panel owns the state; the popup is a
        // stateless renderer that returns the click outcome.
        if let Some(state) = self.completion_state() {
            match CompletionPopup::show(ui.ctx(), &state, &self.instance) {
                CompletionOutcome::Accept(index) => {
                    self.accept_completion_index(index);
                }
                CompletionOutcome::Dismiss => self.close_completion(),
                CompletionOutcome::None => {}
            }
        }

        // Render the hover tooltip (a no-op when closed).
        if let Some(state) = self.hover_state() {
            match HoverTooltip::show(ui.ctx(), &state, &self.instance) {
                HoverOutcome::GotoDefinition(line) => {
                    // MT-052 jump-history record site #2b (hover "Go to definition" link — same
                    // goto-definition jump class): record the pre-jump location before the caret moves.
                    self.record_jump_origin();
                    self.navigate_to_line(line);
                    self.close_hover();
                }
                HoverOutcome::None => {}
            }
        }

        // MT-047: drain a delivered signature-help result into the popup state. A delivered result that
        // anchors to a call site the cursor has since left is dropped (the cursor-exit check) so a stale
        // popup does not linger.
        if let Some(state) =
            self.signature_help_result.lock().unwrap_or_else(|e| e.into_inner()).take()
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
            let guard = self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner());
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
            self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner()).select_prev();
        }
        if down {
            self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner()).select_next();
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
                let glyph_width = (*self.glyph_width_px.lock().unwrap_or_else(|e| e.into_inner()))
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
                RenameState::Editing { original, draft, .. } => (original.clone(), draft.clone()),
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
        *self.rename_state.lock().unwrap_or_else(|e| e.into_inner()) =
            RenameState::Previewing { workspace_edit: preview };
    }

    /// MT-048: the editor body context menu with the code-pane 'Rename Symbol' entry. A secondary-click
    /// over `rect` opens the menu; clicking 'Rename Symbol' runs the SAME `begin_rename` path as F2. Emits
    /// the `Role::MenuItem` AccessKit node `code_editor_ctx_rename_symbol` (suffixed by instance) so a
    /// swarm agent can trigger the rename by id without a right-click (AC-005 / HBR-SWARM). The node is
    /// emitted EVERY frame (not just while the menu is open) so the swarm surface is always addressable;
    /// an AccessKit `Click` action on it dispatches the rename via the editor-action wiring's command
    /// path the same way F2 does.
    fn render_editor_context_menu(&self, ui: &mut egui::Ui, rect: egui::Rect) {
        // A secondary-click sensing response over the editor body so `context_menu` can open on it.
        let resp = ui.interact(
            rect,
            ui.id().with(("code-editor-ctx-menu", &self.instance)),
            egui::Sense::click(),
        );
        resp.context_menu(|ui| {
            if ui.button("Rename Symbol").clicked() {
                self.begin_rename_at_cursor();
                ui.close();
            }
            // MT-049 (AC-007): the 'Quick Fix...' entry routes to the SAME request+open_menu flow as
            // Ctrl+. — it arms the quick-fix request, which the per-frame pump fires for the caret line and
            // opens the menu (no duplicate apply logic; the controller owns the apply path).
            if ui.button("Quick Fix...").clicked() {
                self.quick_fix_request.store(true, Ordering::Relaxed);
                ui.close();
            }
        });

        // Always-present MenuItem AccessKit node carrying the exact contract author_id (AC-005). A swarm
        // agent reads/activates it by id; the value names the action so a no-context model knows what it
        // does. Emitted on a fixed node id in the rename overlay band, distinct from the popup nodes.
        let author = if self.instance.is_empty() {
            rename::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID.to_owned()
        } else {
            format!(
                "{}#{}",
                rename::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID, self.instance
            )
        };
        let node_id = if self.instance.is_empty() {
            // SAFETY: a single hand-assigned fixed id (715) in the disjoint rename overlay band (above the
            // 710..714 rename popup nodes); never reused, cannot self-collide.
            unsafe { egui::Id::from_high_entropy_bits(715) }
        } else {
            egui::Id::new(format!(
                "{}#{}",
                rename::CODE_EDITOR_CTX_RENAME_SYMBOL_AUTHOR_ID, self.instance
            ))
        };
        ui.ctx().accesskit_node_builder(node_id, move |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_author_id(author.clone());
            node.set_label("Rename Symbol".to_owned());
            node.set_value("Rename the symbol under the cursor (F2)".to_owned());
            node.add_action(accesskit::Action::Click);
        });

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
                code_actions::CODE_EDITOR_CTX_QUICK_FIX_AUTHOR_ID, self.instance
            ))
        };
        ui.ctx().accesskit_node_builder(qf_node_id, move |node| {
            node.set_role(accesskit::Role::MenuItem);
            node.set_author_id(qf_author.clone());
            node.set_label("Quick Fix".to_owned());
            node.set_value("Show code actions / quick fixes for the current line (Ctrl+.)".to_owned());
            node.add_action(accesskit::Action::Click);
        });
    }

    /// Render the MT-006 outline (symbol) tree in the left side panel, with the AccessKit `Role::Tree`
    /// node `code_editor_outline` (AC-004 / HBR-SWARM). Each symbol row is a clickable
    /// `CollapsingHeader`-style entry; clicking it calls [`navigate_to_line`](Self::navigate_to_line)
    /// (fold-aware) to scroll the editor + move the caret to the symbol's line. The list scrolls
    /// (an outline can be long — MT step "use ScrollArea for the outline").
    fn render_outline_panel(&self, ui: &mut egui::Ui, syntax: &HsSyntaxTokens) {
        ui.scope_builder(egui::UiBuilder::new().id_salt(self.outline_panel_scope_id()), |ui| {
            let outline_node_id = ui.unique_id();
            ui.label(egui::RichText::new("OUTLINE").color(syntax.comment).small());
            ui.separator();

            let items = self.outline_items();
            let mut navigate_to: Option<usize> = None;
            egui::ScrollArea::vertical()
                .id_salt(("code-editor-outline-scroll", self.outline_panel_scope_id()))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if items.is_empty() {
                        ui.label(
                            egui::RichText::new("No symbols").italics().color(syntax.comment),
                        );
                    }
                    for (idx, item) in items.iter().enumerate() {
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
                        if resp.clicked() {
                            navigate_to = Some(item.line);
                        }
                        resp.on_hover_text(format!("Go to line {} ({})", item.line + 1, item.kind.label()));
                        // Stable per-row egui id so the row is individually addressable; the row index is
                        // unique per frame (outline order is deterministic). (The container Tree node is
                        // the AC-004 addressable surface; individual rows live in egui's hashed id space,
                        // the same dynamic-row pattern the shell tree/list containers use.)
                        let _ = idx;
                    }
                });

            // Emit the outline Tree node onto this scope's Ui id (AC-004 / HBR-SWARM).
            let author = self.outline_author_id();
            let count = items.len();
            ui.ctx().accesskit_node_builder(outline_node_id, move |node| {
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
        });
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
        ui.scope_builder(egui::UiBuilder::new().id_salt(self.minimap_panel_scope_id()), |ui| {
            let minimap_node_id = ui.unique_id();

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

            let response = self
                .minimap
                .render(ui, &row_colors, visible_buffer_range.clone(), total_lines);
            // Store the minimap's TRUE content rect (exactly the configured width) for the AC-006
            // midpoint-click geometry + AC-003 width assertion — the enclosing SidePanel adds frame
            // margins around this, so the panel's outer rect is wider.
            *self.last_minimap_rect.lock().unwrap_or_else(|e| e.into_inner()) =
                Some(response.content_rect);
            // A minimap click is a scroll-to request, routed through the fold-aware mapping (MT
            // positioning note) so a click lands on the correct row even with folds active.
            if let Some(line) = response.clicked_buffer_line {
                let visible_line = self.buffer_line_to_visible_line(line);
                self.scroll_to_line(visible_line);
            }

            // Emit the minimap ScrollBar node (AC-004 / HBR-SWARM). It MUST carry an author_id — a
            // ScrollBar is an INTERACTIVE role the MT-025 accessibility gate flags if unnamed.
            let author = self.minimap_author_id();
            let value = format!("lines {}-{} of {total_lines}", visible_buffer_range.start, visible_buffer_range.end);
            ui.ctx().accesskit_node_builder(minimap_node_id, move |node| {
                node.set_role(accesskit::Role::ScrollBar);
                node.set_author_id(author.clone());
                node.set_label("Code editor minimap".to_owned());
                node.set_value(value.clone());
            });
        });
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
            let palette = self.symbol_palette.lock().unwrap_or_else(|e| e.into_inner());
            (palette.query().to_owned(), palette.results().to_vec(), palette.selected_index())
        };
        let mut query_changed = false;
        let mut confirm_index: Option<usize> = None;

        let window_id = if self.instance.is_empty() {
            egui::Id::new("code_editor_symbol_palette_window")
        } else {
            egui::Id::new(format!("code_editor_symbol_palette_window#{}", self.instance))
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
                    ui.ctx().accesskit_node_builder(search_node_id, move |node| {
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
            ui.ctx().accesskit_node_builder(dialog_node_id, move |node| {
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
                let mut palette = self.symbol_palette.lock().unwrap_or_else(|e| e.into_inner());
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
        // Recompute headers EVERY frame from the CURRENT scroll offset + the live fold regions (RISK-004 /
        // MC-004 — no caching across edits). The first visible BUFFER line is the start of the last painted
        // buffer-line window (`show` captured it last frame; on the first frame it is 0..0 -> top line 0).
        let viewport_top = self.last_painted_buffer_range(total_lines).start;
        let fold_set = self.fold_set();
        let headers = {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            self.sticky_scroll.compute(viewport_top, &fold_set.regions, &buffer)
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
                                egui::RichText::new(text).monospace().color(header_text_color),
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
                ui.ctx().accesskit_node_builder(container_node_id, move |node| {
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
            return;
        };

        // Pin to the top-right corner of the editor area (VS Code style — a floating widget, not a side
        // panel — MT step 6). Width 400 px, height grows with the replace row.
        let bar_width = 400.0_f32.min(panel_rect.width().max(120.0));
        let bar_height = if state.show_replace { 64.0 } else { 34.0 };
        let bar_min = egui::pos2(panel_rect.right() - bar_width - 4.0, panel_rect.top() + 4.0);
        let bar_rect = egui::Rect::from_min_size(bar_min, egui::vec2(bar_width, bar_height));

        let mut query_changed = false;
        let mut close_requested = false;

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

                if ui.button("\u{2191}").on_hover_text("Previous match").clicked() {
                    self.prev_match();
                }
                if ui.button("\u{2193}").on_hover_text("Next match").clicked() {
                    self.next_match();
                }
                ui.label(self.find_state().map(|s| s.counter_label()).unwrap_or_default());
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
                    let _ = ui.add(
                        egui::TextEdit::singleline(&mut state.replace_text)
                            .id_salt(("code-editor-replace-input", self.text_id()))
                            .desired_width(150.0)
                            .hint_text("Replace"),
                    );
                    if ui.button("Replace").clicked() {
                        self.set_replace_text(state.replace_text.clone());
                        self.replace_current();
                    }
                    if ui.button("Replace All").clicked() {
                        self.set_replace_text(state.replace_text.clone());
                        self.replace_all();
                    }
                });
            }
        });

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

        let find_author = self.suffixed(CODE_EDITOR_FIND_BAR_AUTHOR_ID);
        ui.ctx().accesskit_node_builder(
            self.find_node_id(PANEL_FIND_BAR_NODE_ID, CODE_EDITOR_FIND_BAR_AUTHOR_ID),
            move |node| {
                // DEVIATION (API-correct): the contract names `Role::SearchBox`, which does not exist
                // in accesskit 0.21; `Role::SearchInput` is the field-correct search-input role.
                node.set_role(accesskit::Role::SearchInput);
                node.set_author_id(find_author.clone());
                node.set_label("Code editor find".to_owned());
            },
        );

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
        let mut cached = self.line_height_px.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(h) = *cached {
            return h;
        }
        let h = ui.text_style_height(&egui::TextStyle::Monospace).max(1.0);
        *cached = Some(h);
        h
    }

    /// Measure + cache the monospace glyph advance width (px), measured with the EXACT
    /// `FontId::monospace(MONO_FONT_SIZE)` that `render_line` paints glyphs with — so a caret at column
    /// `c` lands on column `c`'s glyph with no x-unit drift (MT-003 positioning requirement /
    /// implementation note 4). All monospace glyphs share one advance, so the space ' ' is
    /// representative. Falls back to half the line height if a font measurement is unavailable.
    fn glyph_width(&self, ui: &egui::Ui) -> f32 {
        let mut cached = self.glyph_width_px.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(w) = *cached {
            return w;
        }
        let font = egui::FontId::monospace(MONO_FONT_SIZE);
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
        ui.scope_builder(egui::UiBuilder::new().id_salt(text_id), |ui| {
            let text_node_id = ui.unique_id();
            ui.style_mut().spacing.item_spacing.y = 0.0;

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
            for visible_idx in row_range.start..visible_end {
                let buffer_line = {
                    let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
                    set.visible_line_to_buffer_line(visible_idx)
                };
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

            let end = buffer_end; // alias kept for the overlay calls below (buffer-line exclusive end).

            // MT-054: paint the editor-chrome decorations (indent guides, bracket-pair colorization,
            // matching-bracket highlight) over the painted rows in the contract z-order: indent guides
            // first (faint lines that sit in the whitespace columns, below the glyphs), then re-draw each
            // bracket glyph in its depth color, then the matching-bracket highlight box. All theme-sourced
            // (CONTROL-4 — colors come from the palette tokens, never a hex literal here). Each visible
            // row in this non-wrap path is one logical line, so every row is a `wrap_index == 0` first
            // fragment and carries its indent guides (RISK-007 trivially holds when wrap is off).
            self.paint_chrome_decorations(ui, &geometry, glyph_width, first_buffer_line, end, None);

            // MT-004: paint the find-match highlights (below the carets) so a caret/selection stays
            // visible on top of a match rect. Restricted to the painted row window (the same sans-spacing
            // line_height + monospace glyph_width units as the cursor overlay).
            self.paint_match_highlights(ui, &geometry, glyph_width, end);

            // MT-003: paint every caret + selection as a painter overlay OVER the rows, restricted to
            // the painted row window so carets align exactly with rendered glyphs (no draw for cursors
            // scrolled off-screen). Uses `y_for_line`-equivalent units (line * line_height) + the same
            // monospace glyph width.
            self.paint_cursor_overlay(ui, &geometry, glyph_width, end, syntax);

            // Emit the TextInput node onto this nested scope's Ui id (AC-005). Because this scope is a
            // child of the scroll-area scope (itself a child of the container), the node is a
            // descendant of the container node.
            let author = text_author.to_owned();
            ui.ctx().accesskit_node_builder(text_node_id, move |node| {
                node.set_role(accesskit::Role::TextInput);
                node.set_author_id(author.clone());
                node.set_label("Code editor text".to_owned());
                node.set_value(format!("{total_lines} lines"));
            });

            // MT-003 AC-004: emit one `Role::Caret` AccessKit node per cursor (capped at
            // MAX_ACCESSKIT_CURSORS — RISK-004 / MC-004), nested under the text node so a swarm agent
            // can address each caret by `code_editor_cursor_{n}`. (The contract named `Role::TextCursor`,
            // which does not exist in accesskit 0.21; `Role::Caret` is the field-correct caret role —
            // see `emit_cursor_nodes` for the documented deviation.)
            self.emit_cursor_nodes(ui);

            // MT-005 AC-005: emit one `Role::TreeItem` AccessKit node per foldable region intersecting
            // the painted buffer window (capped at MAX_ACCESSKIT_FOLDS — RISK-001), with an
            // Expand/Collapse action reflecting the fold state, so a swarm agent can fold/unfold each
            // region by `code_editor_fold_{start_line}`. Nested under the text node like the cursors.
            self.emit_fold_nodes(ui, first_buffer_line, buffer_end);
        });
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
        let mut guard = self.wrap_row_index.lock().unwrap_or_else(|e| e.into_inner());
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
        let guard = self.wrap_row_index.lock().unwrap_or_else(|e| e.into_inner());
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
        ui.scope_builder(egui::UiBuilder::new().id_salt(text_id), |ui| {
            let text_node_id = ui.unique_id();
            ui.style_mut().spacing.item_spacing.y = 0.0;
            let origin = ui.cursor().min;

            // The buffer-line span the painted visual rows cover, for the highlight-span byte window.
            let (first_buffer_line, last_buffer_line) = if !window_rows.is_empty() {
                (window_rows[0].logical_line, window_rows[window_rows.len() - 1].logical_line)
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
            // match highlight mapped through the painted visual-row window (RISK-007 / MC-007).
            self.paint_chrome_decorations(
                ui,
                &geometry,
                glyph_width,
                first_buffer_line,
                buffer_end,
                Some(window_rows),
            );

            let author = text_author.to_owned();
            ui.ctx().accesskit_node_builder(text_node_id, move |node| {
                node.set_role(accesskit::Role::TextInput);
                node.set_author_id(author.clone());
                node.set_label("Code editor text".to_owned());
                node.set_value(format!("{total_lines} lines (word wrap on)"));
            });
        });
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
        visible_spans: &[HighlightSpan],
        syntax: &HsSyntaxTokens,
    ) {
        let frag_start = row.byte_start;
        let frag_text_owned = self.with_buffer(|b| b.byte_slice_to_string(row.byte_range()));
        let frag_text = frag_text_owned.strip_suffix('\n').unwrap_or(&frag_text_owned);
        let frag_end = frag_start + frag_text.len();

        let mut runs: Vec<(std::ops::Range<usize>, HighlightScope)> = Vec::new();
        for span in visible_spans {
            let s = span.byte_range.start.max(frag_start);
            let e = span.byte_range.end.min(frag_end);
            if s < e {
                runs.push((s..e, span.scope));
            }
        }
        runs.sort_by_key(|(r, _)| r.start);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mono = egui::FontId::monospace(MONO_FONT_SIZE);
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
                        ui.label(egui::RichText::new(gap).font(mono.clone()).color(default_color));
                    }
                }
                let run_text = frag_slice(range.start, range.end);
                if !run_text.is_empty() {
                    let color = scope_to_color(*scope, syntax);
                    ui.label(egui::RichText::new(run_text).font(mono.clone()).color(color));
                }
                cursor = cursor.max(range.end);
            }
            if cursor < frag_end {
                let tail = frag_slice(cursor, frag_end);
                if !tail.is_empty() {
                    ui.label(egui::RichText::new(tail).font(mono.clone()).color(default_color));
                }
            }
            if runs.is_empty() && frag_text.is_empty() {
                ui.label(egui::RichText::new(" ").font(mono.clone()).color(default_color));
            }
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
        first_buffer_line: usize,
        end_line: usize,
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
        match wrap_rows {
            None => {
                // Buffer lines are contiguous in the non-wrap painted window EXCEPT across folded
                // regions; we map each buffer line in range to its row offset by counting from
                // first_buffer_line. A folded gap simply leaves that row offset unused (the guide for a
                // hidden line is never drawn — correct).
                for (row_offset, line) in (first_buffer_line..end_line).enumerate() {
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

        // 2) BRACKET-PAIR COLORIZATION: re-draw each bracket glyph in its depth color over the painted
        //    text (z-order: above guides + text). Computed over the painted buffer byte window.
        let (win_start, win_end) = {
            let ws = buffer.line_to_byte(first_buffer_line).unwrap_or(0);
            let we = buffer
                .line_to_byte(end_line)
                .unwrap_or_else(|| buffer.len_bytes());
            (ws, we)
        };
        if !bracket_palette.is_empty() {
            let colors = bracket_pair_colors(&buffer, win_start..win_end, &bracket_palette);
            let mono = egui::FontId::monospace(MONO_FONT_SIZE);
            for (range, color) in colors {
                if let Some((x, y)) =
                    self.decoration_xy(&buffer, range.start, geometry, glyph_width, wrap_rows)
                {
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
        //    adjacent to a bracket (VS Code adjacency). Painted last so it sits on top.
        let cursor_byte = {
            let set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            set.primary().head
        };
        if let Some(BracketMatch { open_byte, close_byte }) =
            find_matching_bracket(&buffer, cursor_byte)
        {
            let stroke = egui::Stroke::new(1.0, active_guide_color);
            for b in [open_byte, close_byte] {
                if let Some((x, y)) =
                    self.decoration_xy(&buffer, b, geometry, glyph_width, wrap_rows)
                {
                    let rect = egui::Rect::from_min_size(
                        egui::pos2(x, y),
                        egui::vec2(glyph_width, geometry.line_height),
                    );
                    painter.rect_stroke(
                        rect,
                        2.0,
                        stroke,
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
    }

    /// MT-054: map an absolute buffer byte offset to the (x, y) top-left of its glyph in the painted row
    /// window, or `None` if the offset is not on a painted row. Non-wrap: the row y is the buffer line
    /// offset from `geometry.first_line`; the column is the char offset within the line. Wrap: find the
    /// visual row whose byte fragment contains the offset and use its visual-row index for y + the offset
    /// within the fragment for the column. Reuses the SAME `glyph_width` + `line_height` units the rows
    /// were painted with (RISK-002 / MC-002 — no independent metric recompute).
    fn decoration_xy(
        &self,
        buffer: &TextBuffer,
        byte_offset: usize,
        geometry: &RowGeometry,
        glyph_width: f32,
        wrap_rows: Option<&[VisualRow]>,
    ) -> Option<(f32, f32)> {
        match wrap_rows {
            None => {
                let (line, col) = byte_to_line_col(byte_offset, buffer);
                if line < geometry.first_line {
                    return None;
                }
                let row_offset = line - geometry.first_line;
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

    /// Render a folded region's collapsed SUMMARY line (the start-line text + ` …`) in place of the
    /// region's real lines (MT step 4). One row, monospace, in the editor foreground color — the same
    /// row height as a real line so the virtualized layout stays on one unit. A subtle background tint
    /// (the theme comment color at low alpha) marks it as a fold summary without a new theme token.
    fn render_fold_label_line(&self, ui: &mut egui::Ui, label: &str, syntax: &HsSyntaxTokens) {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mono = egui::FontId::monospace(13.0);
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

    /// Emit the per-fold-region `Role::TreeItem` AccessKit nodes for the regions whose start line falls
    /// in the painted buffer window `[first_buffer_line, buffer_end)` (AC-005 / HBR-SWARM). Each node:
    /// - author_id `code_editor_fold_{start_line}` (the contract-named id; AC-005 asserts THIS id),
    /// - role `Role::TreeItem` (exists in accesskit 0.21 — no fallback needed; verified at build),
    /// - action `Action::Expand` when the region is FOLDED (the agent action that unfolds it) or
    ///   `Action::Collapse` when UNFOLDED (the agent action that folds it) — MT impl note "accessible
    ///   fold state",
    /// - value carries the fold state + line span so an agent can read it without dispatching.
    ///
    /// Capped at [`MAX_ACCESSKIT_FOLDS`] nodes (RISK-001) so a file with thousands of folds cannot blow
    /// the per-frame node budget. Fixed ids in the fold band (default panel) keep NodeIds stable across
    /// frames; instances hash the suffixed author_id (RISK-004), the same scheme the cursor nodes use.
    fn emit_fold_nodes(&self, ui: &egui::Ui, first_buffer_line: usize, buffer_end: usize) {
        let regions: Vec<(usize, usize, bool)> = {
            let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            set.regions
                .iter()
                .filter(|r| r.start_line >= first_buffer_line && r.start_line < buffer_end)
                .take(MAX_ACCESSKIT_FOLDS)
                .map(|r| (r.start_line, r.end_line, r.folded))
                .collect()
        };
        for (slot, (start_line, end_line, folded)) in regions.into_iter().enumerate() {
            let author = if self.instance.is_empty() {
                format!("{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}")
            } else {
                format!("{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}#{}", self.instance)
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
        }
    }

    /// The fixed `egui::Id` for fold node `slot` (default panel uses the fold band; instances hash the
    /// suffixed author_id so two panels never share a fold id — RISK-004).
    fn fold_node_id(&self, slot: usize, start_line: usize) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: each slot maps to a distinct fixed id in the disjoint fold band; never reused.
            unsafe { egui::Id::from_high_entropy_bits(PANEL_FOLD_NODE_ID_BASE + slot as u64) }
        } else {
            egui::Id::new(format!("{CODE_EDITOR_FOLD_AUTHOR_PREFIX}{start_line}#{}", self.instance))
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
    ) {
        // The painted-row geometry captured by `render_rows` this frame (origin = top-left of the first
        // painted code row; line_height = sans-spacing row stride). Without it (no frame yet) there is
        // nothing to align to.
        let Some(row_geom) = *self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()) else {
            self.last_gutter_rows.lock().unwrap_or_else(|e| e.into_inner()).clear();
            *self.last_gutter_geometry.lock().unwrap_or_else(|e| e.into_inner()) = None;
            return;
        };

        // The painted VISIBLE-line window (post-fold), mapped to BUFFER lines per row so the gutter draws
        // the right line numbers/markers in the right rows. The gutter strip starts at the gutter rect's
        // left edge but uses the editor rows' TOP (row_geom.top) so the first gutter row lines up with
        // the first code row exactly.
        let visible_range = self.last_visible_range();
        let visible_rows: Vec<usize> = {
            let mut set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            (visible_range.start..visible_range.end)
                .map(|v| set.visible_line_to_buffer_line(v))
                .collect()
        };

        // The gutter geometry: origin at the gutter strip's left edge + the code rows' top, with the
        // editor's measured line height + glyph width (so the line numbers use the SAME metrics).
        let geometry = GutterGeometry {
            origin: egui::pos2(gutter_rect.left(), row_geom.top),
            line_height: row_geom.line_height,
            char_width: glyph_width,
        };

        // Snapshot the markers + breakpoints (clones so no lock is held across egui calls).
        let markers = self.diagnostic_markers.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let breakpoints = self.breakpoint_set.lock().unwrap_or_else(|e| e.into_inner()).clone();

        // A closure the gutter calls to learn whether a buffer line starts a fold region and, if so,
        // whether it is OPEN (not folded). `Some(true)` = region start, expanded; `Some(false)` = region
        // start, collapsed; `None` = not a region start (no triangle).
        let fold_open_for = |line: usize| -> Option<bool> {
            let set = self.fold_set.lock().unwrap_or_else(|e| e.into_inner());
            set.region_starting_at(line).map(|r| !r.folded)
        };

        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let response: GutterResponse = Gutter::render(
            ui,
            gutter_rect,
            &visible_rows,
            &buffer,
            &markers,
            &breakpoints,
            config,
            geometry,
            &fold_open_for,
        );

        // Persist the painted rows + geometry for the deterministic click tests.
        *self.last_gutter_rows.lock().unwrap_or_else(|e| e.into_inner()) = visible_rows.clone();
        *self.last_gutter_geometry.lock().unwrap_or_else(|e| e.into_inner()) = Some(geometry);

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
        self.emit_breakpoint_nodes(ui, &visible_rows, &breakpoints, config);
        self.emit_diagnostic_nodes(ui, &visible_rows, &markers, config);

        // MT-049: draw the quick-fix lightbulb on any PAINTED line that currently has available code
        // actions (AC-003 — only on the diagnostic line with actions, never on a line without). The glyph is
        // a clickable Role::Button that opens the quick-fix menu (the gutter-click path). Theme-aware
        // (CONTROL-4 — `lightbulb_color`/`warn_fg_color`, no Color32 literal). Only drawn when the menu is
        // closed so the bulb does not overdraw the open menu (the bulb stays "lit" via the controller state).
        self.draw_quickfix_lightbulbs(ui, &visible_rows, &geometry);
    }

    /// MT-049: draw the quick-fix lightbulb on each painted line that carries available code actions
    /// (AC-003). A click on the bulb opens the quick-fix menu (the gutter-click trigger). The bulb sits in
    /// the gutter's left margin, vertically centered on its row. Only the painted lines are considered so a
    /// huge file cannot draw an off-screen bulb. A clicked bulb opens the menu for that line.
    fn draw_quickfix_lightbulbs(
        &self,
        ui: &mut egui::Ui,
        visible_rows: &[usize],
        geometry: &GutterGeometry,
    ) {
        let mut open_for: Option<usize> = None;
        for (painted_idx, &line) in visible_rows.iter().enumerate() {
            if !self.has_quickfix_on_line(line) {
                continue;
            }
            // Center the bulb on the row, near the gutter's left edge (left of the line number / diagnostic
            // glyph column). The origin.x is the gutter strip left; offset a half-glyph in so it has margin.
            let y = geometry.origin.y
                + painted_idx as f32 * geometry.line_height
                + geometry.line_height * 0.5;
            let x = geometry.origin.x + geometry.char_width * 0.6;
            let pos = egui::pos2(x, y);
            let resp = code_actions::draw_lightbulb(ui, line, pos, &self.instance);
            if resp.clicked() {
                open_for = Some(line);
            }
        }
        // A clicked lightbulb opens the quick-fix menu for that line (AC-003 — gutter-click path). If a
        // request for the line is not yet resolved the menu opens empty and re-fires on the next pump.
        if let Some(line) = open_for {
            let mut controller =
                self.code_action_controller.lock().unwrap_or_else(|e| e.into_inner());
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
        let value = if enabled { "word wrap on" } else { "word wrap off" }.to_owned();
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
            unsafe {
                egui::Id::from_high_entropy_bits(PANEL_BREAKPOINT_NODE_ID_BASE + slot as u64)
            }
        } else {
            egui::Id::new(format!("{CODE_EDITOR_BREAKPOINT_AUTHOR_PREFIX}{line}#{}", self.instance))
        }
    }

    /// The fixed `egui::Id` for diagnostic node `slot` (default panel uses the diagnostic band; instances
    /// hash the suffixed author_id — RISK-004).
    fn diagnostic_node_id(&self, slot: usize, line: usize) -> egui::Id {
        if self.instance.is_empty() {
            // SAFETY: each slot maps to a distinct fixed id in the disjoint diagnostic band; never reused.
            unsafe {
                egui::Id::from_high_entropy_bits(PANEL_DIAGNOSTIC_NODE_ID_BASE + slot as u64)
            }
        } else {
            egui::Id::new(format!("{CODE_EDITOR_DIAGNOSTIC_AUTHOR_PREFIX}{line}#{}", self.instance))
        }
    }

    /// Clip the sorted cached span list to the half-open byte window `[win_start, win_end)`, returning
    /// just the spans that can overlap it. The cache is sorted by start byte, so a binary search finds
    /// the first span that could reach into the window; from there a forward scan collects spans until
    /// one starts past the window end. This bounds per-frame span work to the visible window rather
    /// than the whole document (MT-002 step 3). Spans are cloned out so the cache lock is not held
    /// across the egui layout calls in `render_line`.
    fn spans_in_byte_window(&self, win_start: usize, win_end: usize) -> Vec<HighlightSpan> {
        if win_end <= win_start {
            return Vec::new();
        }
        let cache = self.highlight_cache.lock().unwrap_or_else(|e| e.into_inner());
        let Some((spans, _)) = cache.as_ref() else {
            return Vec::new();
        };
        // A span [s.start, s.end) overlaps the window iff s.start < win_end AND s.end > win_start.
        // Spans are sorted by (start, end). The earliest span that can overlap is the first whose
        // `end > win_start`; but `end` is not the sort key, so we cannot binary-search on it directly.
        // Instead, find the first span with `start >= win_start` (lower bound on start) and step a
        // little backward to include a span that started before the window but extends into it (a
        // multi-line comment/string). A bounded back-scan is enough because spans here are token-sized
        // except rare block comments; cap it so a pathological input cannot make this O(n).
        let lb = spans.partition_point(|s| s.byte_range.start < win_start);
        // Back up over spans whose end still reaches into the window (e.g. a block comment opened above
        // the viewport). Bounded so worst case stays cheap.
        let mut begin = lb;
        let mut backstep = 0usize;
        const MAX_BACKSTEP: usize = 4096;
        while begin > 0 && backstep < MAX_BACKSTEP && spans[begin - 1].byte_range.end > win_start {
            begin -= 1;
            backstep += 1;
        }
        let mut out = Vec::new();
        for s in &spans[begin..] {
            if s.byte_range.start >= win_end {
                break; // sorted by start: nothing further can overlap.
            }
            if s.byte_range.end > win_start {
                out.push(s.clone());
            }
        }
        out
    }

    /// Render one line as a sequence of theme-colored runs, splitting the line text at the highlight
    /// span boundaries that overlap it. `visible_spans` is the per-frame window-clipped span slice (so
    /// this is O(spans-in-window), not O(all-spans)). A line with no overlapping spans renders as plain
    /// foreground text. Byte->char conversions go through the buffer (RISK-002).
    fn render_line(
        &self,
        ui: &mut egui::Ui,
        line_idx: usize,
        visible_spans: &[HighlightSpan],
        syntax: &HsSyntaxTokens,
    ) {
        let (line_text_owned, line_start_byte) = self.with_buffer(|b| {
            (b.slice_to_string(line_idx..line_idx + 1), b.line_to_byte(line_idx).unwrap_or(0))
        });
        // Strip the trailing newline so each visual line is one row (the layout adds the row break).
        let line_text = line_text_owned.strip_suffix('\n').unwrap_or(&line_text_owned);
        let line_end_byte = line_start_byte + line_text.len();

        // Spans overlapping THIS line, clipped to the line's byte window (from the already
        // window-clipped frame slice).
        let mut runs: Vec<(std::ops::Range<usize>, HighlightScope)> = Vec::new();
        for span in visible_spans {
            let s = span.byte_range.start.max(line_start_byte);
            let e = span.byte_range.end.min(line_end_byte);
            if s < e {
                runs.push((s..e, span.scope));
            }
        }
        runs.sort_by_key(|(r, _)| r.start);

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let mono = egui::FontId::monospace(13.0);
            let default_color = syntax.punctuation;
            let mut cursor = line_start_byte;

            // Helper to slice a [start,end) byte window of the line into a &str safely (RISK-002:
            // respect char boundaries; fall back to empty on a bad boundary).
            let line_slice = |start: usize, end: usize| -> String {
                let rel_start = start.saturating_sub(line_start_byte);
                let rel_end = end.saturating_sub(line_start_byte);
                if rel_start >= rel_end || rel_end > line_text.len() {
                    return String::new();
                }
                let bytes = line_text.as_bytes();
                let mut a = rel_start;
                while a < line_text.len() && !line_text.is_char_boundary(a) {
                    a += 1;
                }
                let mut b = rel_end.min(line_text.len());
                while b < line_text.len() && !line_text.is_char_boundary(b) {
                    b += 1;
                }
                if a >= b {
                    return String::new();
                }
                std::str::from_utf8(&bytes[a..b]).unwrap_or("").to_owned()
            };

            for (range, scope) in &runs {
                if range.start > cursor {
                    // Plain (un-highlighted) gap before this run.
                    let gap = line_slice(cursor, range.start);
                    if !gap.is_empty() {
                        ui.label(egui::RichText::new(gap).font(mono.clone()).color(default_color));
                    }
                }
                let run_text = line_slice(range.start, range.end);
                if !run_text.is_empty() {
                    let color = scope_to_color(*scope, syntax);
                    ui.label(egui::RichText::new(run_text).font(mono.clone()).color(color));
                }
                cursor = cursor.max(range.end);
            }
            // Trailing plain text after the last run.
            if cursor < line_end_byte {
                let tail = line_slice(cursor, line_end_byte);
                if !tail.is_empty() {
                    ui.label(egui::RichText::new(tail).font(mono.clone()).color(default_color));
                }
            }
            // Empty line: emit a zero-width spacer so the row still occupies a line height.
            if runs.is_empty() && line_text.is_empty() {
                ui.label(egui::RichText::new(" ").font(mono.clone()).color(default_color));
            }
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
        end_line: usize,
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
        let y_for =
            |line: usize| geometry.top + (line - geometry.first_line) as f32 * geometry.line_height;

        for (idx, m) in state.matches.iter().enumerate() {
            let color = if idx == state.current_match {
                CURRENT_MATCH_HIGHLIGHT_COLOR
            } else {
                MATCH_HIGHLIGHT_COLOR
            };
            // The match's start/end (line, col). A match is usually single-line; a multi-line regex
            // match is handled by drawing one rect per covered line (start col on the first line, end
            // col on the last, whole content width between).
            let (start_line, start_col) = byte_to_line_col(m.byte_range.start, &buffer);
            let (end_match_line, end_col) = byte_to_line_col(m.byte_range.end, &buffer);
            for line in start_line..=end_match_line {
                if line < geometry.first_line || line >= end_line {
                    continue; // off-screen row (implementation note 2)
                }
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
                let y0 = y_for(line);
                let rect = egui::Rect::from_min_max(
                    egui::pos2(x0, y0),
                    egui::pos2(x1, y0 + geometry.line_height),
                );
                painter.rect_filled(rect, 0.0, color);
            }
        }
    }

    // ── MT-003 overlay + AccessKit + input ───────────────────────────────────────────────────────────

    /// Paint every caret (a 2px vertical bar) and every selection (a semi-transparent rect) over the
    /// painted rows, restricted to lines `geometry.first_line..end_line` (the on-screen window) so a
    /// caret only draws where its glyph is actually rendered (MT-003 step 6). A selection that spans
    /// multiple lines is drawn as one rect per line in the span (so a box/column selection naturally
    /// shows one rect per row). Column->x uses `glyph_width`; line->y uses `(line - first_line) *
    /// line_height` from the captured geometry (the MT-002 sans-spacing unit).
    fn paint_cursor_overlay(
        &self,
        ui: &egui::Ui,
        geometry: &RowGeometry,
        glyph_width: f32,
        end_line: usize,
        syntax: &HsSyntaxTokens,
    ) {
        let painter = ui.painter();
        // Caret color: the editor foreground (theme-sourced, never a hex literal). Selection overlay is
        // the MT-named cornflower-blue at low alpha — a fixed selection-highlight tint that is NOT a
        // syntax-token color (it is a UI affordance, like egui's own selection bg), so it is the one
        // place the MT contract specifies an explicit RGBA. Kept exactly as the contract names it.
        let caret_color = syntax.punctuation;
        let selection_color = egui::Color32::from_rgba_unmultiplied(100, 149, 237, 80);

        let cursors = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).clone();
        let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());

        let x_for = |col: usize| geometry.left + col as f32 * glyph_width;
        let y_for = |line: usize| geometry.top + (line - geometry.first_line) as f32 * geometry.line_height;

        for cursor in cursors.cursors() {
            // Draw the SELECTION (if any) first, then the caret on top, so the caret stays visible.
            if cursor.is_selection() {
                let range = cursor.range();
                let (start_line, start_col) = byte_to_line_col(range.start, &buffer);
                let (end_line_sel, end_col) = byte_to_line_col(range.end, &buffer);
                for line in start_line..=end_line_sel {
                    if line < geometry.first_line || line >= end_line {
                        continue; // off-screen row
                    }
                    // Column span on THIS line: from the selection start col (or 0 if the line is not the
                    // first) to the end col (or the line's content end if the line is not the last).
                    let line_start_col = if line == start_line { start_col } else { 0 };
                    let line_end_col = if line == end_line_sel {
                        end_col
                    } else {
                        // Whole-line selection rows extend to the line's char length.
                        let (_, content_end_col) =
                            byte_to_line_col(line_col_to_byte(line, usize::MAX, &buffer), &buffer);
                        content_end_col.max(line_start_col + 1) // at least 1 col wide so it is visible
                    };
                    if line_end_col <= line_start_col {
                        continue;
                    }
                    let x0 = x_for(line_start_col);
                    let x1 = x_for(line_end_col);
                    let y0 = y_for(line);
                    let rect = egui::Rect::from_min_max(
                        egui::pos2(x0, y0),
                        egui::pos2(x1, y0 + geometry.line_height),
                    );
                    painter.rect_filled(rect, 0.0, selection_color);
                }
            }
            // Draw the caret (the moving head) as a 2px vertical bar.
            let (head_line, head_col) = byte_to_line_col(cursor.head, &buffer);
            if head_line >= geometry.first_line && head_line < end_line {
                let x = x_for(head_col);
                let y = y_for(head_line);
                let caret = egui::Rect::from_min_max(
                    egui::pos2(x, y),
                    egui::pos2(x + 2.0, y + geometry.line_height),
                );
                painter.rect_filled(caret, 0.0, caret_color);
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
        let cursors = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).clone();
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
            egui::Id::new(format!("{CODE_EDITOR_CURSOR_AUTHOR_PREFIX}{n}#{}", self.instance))
        }
    }

    /// The SCREEN position of the top-left of `(line, col)` from the most recent painted frame, or
    /// `None` if that line is outside the painted window (or no frame has rendered). The deterministic
    /// inverse of [`pointer_to_byte`](Self::pointer_to_byte): a kittest test computes the exact pixel to
    /// inject an Alt+Click at so the click lands on a known cell (AC-004). Adds half a glyph so the
    /// click lands inside the cell, not on its left edge.
    pub fn screen_pos_for_line_col(&self, line: usize, col: usize, glyph_width: f32) -> Option<egui::Pos2> {
        let g = (*self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()))?;
        if line < g.first_line {
            return None;
        }
        let x = g.left + col as f32 * glyph_width + glyph_width * 0.25;
        let y = g.top + (line - g.first_line) as f32 * g.line_height + g.line_height * 0.5;
        Some(egui::pos2(x, y))
    }

    /// The cached monospace glyph width measured on the last `show` (px), or `None` before the first
    /// frame. Lets a test compute click pixels with the SAME width the overlay uses.
    pub fn measured_glyph_width(&self) -> Option<f32> {
        *self.glyph_width_px.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Map a screen position to a buffer byte offset using the captured row geometry (MT-003 pointer
    /// hit-testing). Returns `None` if no geometry is captured yet (no frame painted). Clamps the
    /// column to the clicked line's length (RISK-002) so a click past the line end lands at the line
    /// end, never past it.
    fn pointer_to_byte(&self, pos: egui::Pos2, glyph_width: f32, total_lines: usize) -> Option<usize> {
        let geometry = (*self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()))?;
        if geometry.line_height <= 0.0 || glyph_width <= 0.0 {
            return None;
        }
        // Line = first_line + floor((y - top) / line_height), clamped to the document.
        let rel_y = (pos.y - geometry.top).max(0.0);
        let line = (geometry.first_line + (rel_y / geometry.line_height).floor() as usize)
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
        self.keymap.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
    pub fn set_command_palette_sender(&self, tx: mpsc::Sender<CodeEditorAction>) {
        *self.command_palette_tx.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);
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
        self.pending_chord.lock().unwrap_or_else(|e| e.into_inner()).is_some()
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
        self.suffixed(&format!("{CODE_EDITOR_COMMAND_AUTHOR_PREFIX}{}", action.name()))
    }

    /// Dispatch an editor command by its AccessKit `code_editor_cmd_*` author_id — the path a swarm
    /// agent (via AccessKit `Action::Click` on the hidden node) or an MCP swarm tool takes to drive the
    /// editor WITHOUT simulating a keystroke (AC-005 / HBR-SWARM). Resolves the author_id to its
    /// [`CodeEditorAction`] through the cached command-node descriptors and dispatches it. Returns the
    /// dispatched action, or `None` for an unknown author_id (so a bad id is a no-op, not a panic).
    pub fn dispatch_command_by_author_id(&self, author_id: &str) -> Option<CodeEditorAction> {
        self.ensure_command_nodes();
        let action = {
            let cache = self.command_node_cache.lock().unwrap_or_else(|e| e.into_inner());
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
        *self.editor_action_wiring.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(EditorActionWiring { registry, handle });
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
        let wiring = self.editor_action_wiring.lock().unwrap_or_else(|e| e.into_inner());
        let Some(wiring) = wiring.as_ref() else {
            return Vec::new();
        };
        let handle = wiring.handle;
        let find_state = self.find_state();
        let find_open = find_state.is_some();
        let multi_cursor = self.cursor_count() > 1;
        // 1) Register/refresh every catalog node with its live state.
        {
            use crate::accessibility::editor_action_registry::AxRole;
            let mut reg = wiring.registry.lock().unwrap_or_else(|e| e.into_inner());
            for entry in CODE_ACTION_CATALOG {
                let author_id = handle.author_id(entry.action_id);
                let state = self.code_action_state(entry, find_open, find_state.as_ref(), multi_cursor);
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
                    EditorActionState { present: true, enabled: false, checked: None }
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
                    let action_id = Self::strip_code_author_prefix(aid, handle);
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
            "find-next" | "find-prev" | "find-toggle-case" | "find-toggle-word" | "find-toggle-regex"
                | "replace-one" | "replace-all"
        );
        let present = if find_scoped { find_open } else { entry.always_present };
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
            AxRole::Button => EditorActionState { present, enabled, checked: None },
            AxRole::ToggleButton => {
                // The find option toggles reflect the live FindQuery state (RISK-041-03).
                let checked = find_state.map(|f| match entry.action_id {
                    "find-toggle-case" => f.query.case_sensitive,
                    "find-toggle-word" => f.query.whole_word,
                    "find-toggle-regex" => f.query.is_regex,
                    _ => false,
                });
                EditorActionState { present, enabled, checked }
            }
        }
    }

    /// Strip the `editor.code.` prefix (and the optional `.<idx>` instance suffix) from a canonical
    /// author_id, returning the bare `<action>` id the catalog keys on.
    fn strip_code_author_prefix(author_id: &str, handle: RegistrationHandle) -> String {
        let rest = author_id.strip_prefix("editor.code.").unwrap_or(author_id);
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
            CodeDispatch::MultiCursorClear => self.dispatch_action(CodeEditorAction::CancelMultiCursor),
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
            let cache = self.command_node_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some((v, _)) = cache.as_ref() {
                if *v == version {
                    return; // up to date for this keymap version.
                }
            }
        }
        let keymap = self.keymap.lock().unwrap_or_else(|e| e.into_inner()).clone();
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
                CommandNodeDesc { node_id, author_id, label, action }
            })
            .collect();
        *self.command_node_cache.lock().unwrap_or_else(|e| e.into_inner()) = Some((version, descs));
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
        let cache = self.command_node_cache.lock().unwrap_or_else(|e| e.into_inner());
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
            let mut state = self.keymap_file_state.lock().unwrap_or_else(|e| e.into_inner());
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
            let mut state = self.keymap_file_state.lock().unwrap_or_else(|e| e.into_inner());
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
        if !matches!(*self.rename_state.lock().unwrap_or_else(|e| e.into_inner()), RenameState::Idle) {
            return;
        }

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
        let keymap = self.keymap.lock().unwrap_or_else(|e| e.into_inner()).clone();

        for event in &events {
            let egui::Event::Key { key, pressed: true, modifiers, .. } = event else {
                continue;
            };
            let chord = KeyChord::from_modifiers(*key, modifiers);

            // 1) If a two-chord prefix is pending, this chord must be the SECOND chord.
            let pending_prefix =
                self.pending_chord.lock().unwrap_or_else(|e| e.into_inner()).map(|(c, _)| c);
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
    fn resolve_contextual(
        &self,
        key: egui::Key,
        modifiers: &egui::Modifiers,
    ) -> ContextOutcome {
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
            } else if self.cursor_set.lock().unwrap_or_else(|e| e.into_inner()).len() > 1 {
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
                self.delete_text();
            }
            A::DeleteRight => self.delete_forward(),
            A::DeleteWordLeft => self.delete_word(true),
            A::DeleteWordRight => self.delete_word(false),
            // MT-051: DeleteLine deletes every affected whole row (incl. trailing newline; the preceding
            // newline too on the last row so no empty trailing line remains) as ONE undo entry.
            A::DeleteLine => {
                self.apply_line_transform("Delete Line", line_ops::delete_line);
            }
            // ── Insertion / line edits ──
            A::InsertNewline => {
                self.insert_text("\n");
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
                self.completion_request.store(true, Ordering::Relaxed);
            }
            A::AcceptCompletion => {
                self.accept_completion();
            }
            A::DismissCompletion => self.close_completion(),
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
    fn delete_forward(&self) {
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
    }

    /// Delete the word to the left (`to_left`) or right of each cursor (Ctrl+Backspace / Ctrl+Delete):
    /// extend each bare caret over the adjacent word, then delete.
    fn delete_word(&self, to_left: bool) {
        {
            let buffer = self.buffer.lock().unwrap_or_else(|e| e.into_inner());
            let mut set = self.cursor_set.lock().unwrap_or_else(|e| e.into_inner());
            set.select_word_for_bare_carets(to_left, &buffer);
        }
        self.delete_text();
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
        let mut changed = false;
        for region in &mut set.regions {
            if region.folded != fold {
                region.folded = fold;
                changed = true;
            }
        }
        if changed {
            // Re-fold/unfold invalidates the cached visible map; rebuilding happens lazily on next query.
            let total = self.with_buffer(|b| b.len_lines());
            set.rebuild_visible_map_for(total);
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
            return; // runtime-less harness: nothing to spawn (graceful).
        };
        let workspace_id = self.workspace_id();
        let word = self.word_at_primary_cursor();
        if workspace_id.is_empty() || word.is_empty() {
            return;
        }
        let client = self.code_nav_client.clone();
        let cell = Arc::clone(&self.goto_def_result);
        runtime.spawn(async move {
            let symbols = client
                .lookup_symbols(&workspace_id, &word, 5)
                .await
                .unwrap_or_default();
            // First matched symbol with a definition span -> 0-based target line (backend is 1-based).
            let target = symbols.into_iter().find_map(|s| {
                s.definition
                    .as_ref()
                    .and_then(|d| d.line_start)
                    .filter(|l| *l >= 1)
                    .map(|l| (l - 1) as usize)
            });
            if let Some(line) = target {
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(line);
                }
            }
        });
    }

    /// Request show-references at the primary caret (Shift+F12). Resolves the symbol under the caret and
    /// calls the MT-008 [`CodeNavClient::get_references`] off-thread, delivering the callers/callees into
    /// [`references_result`](Self::references_result); the next frame moves it into `last_references`
    /// (observable via [`last_references`](Self::last_references)). There is no references-panel UI in
    /// MT-010 scope — rendering it is a follow-on MT (recorded as a typed BLOCKER in the handoff), so this
    /// performs the real backend round-trip without a rendered panel. Graceful no-op without a bound
    /// workspace/runtime or word under the caret.
    fn request_show_references(&self) {
        tracing::debug!("code editor: ShowReferences (Shift+F12) dispatched");
        let Some(runtime) = self.runtime_handle() else {
            return;
        };
        let workspace_id = self.workspace_id();
        let word = self.word_at_primary_cursor();
        if workspace_id.is_empty() || word.is_empty() {
            return;
        }
        let client = self.code_nav_client.clone();
        let cell = Arc::clone(&self.references_result);
        runtime.spawn(async move {
            // Resolve the word to a symbol entity id, then fetch its references.
            let symbols = client
                .lookup_symbols(&workspace_id, &word, 5)
                .await
                .unwrap_or_default();
            let Some(entity_id) = symbols
                .into_iter()
                .map(|s| s.symbol_entity_id)
                .find(|id| !id.is_empty())
            else {
                return; // no resolvable symbol -> no references (graceful).
            };
            if let Ok(refs) = client.get_references(&entity_id).await {
                if let Ok(mut slot) = cell.lock() {
                    *slot = Some(refs);
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

    /// The most recent ShowReferences result (callers + callees), or `None` if no references request has
    /// completed. Observable accessor so tests/agents can confirm the Shift+F12 backend round-trip
    /// (there is no rendered references panel in MT-010 scope).
    pub fn last_references(&self) -> Option<CodeSymbolReferencesResponse> {
        self.last_references.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    // ── MT-034 code->notes cross-references (the NoteRefsPanel live wiring) ─────────────────────────────

    /// Show/hide the "Notes referencing this symbol" panel (RISK-001 / MC-001 — hideable like the
    /// outline/minimap). The toggle button in the editor's panel-toggle row flips it; an agent can too.
    pub fn set_show_note_refs(&self, show: bool) {
        self.show_note_refs.store(show, Ordering::Relaxed);
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
        self.note_refs_state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// The symbol key the NoteRefsPanel currently tracks (the dwelled symbol), or `None`.
    pub fn note_refs_focused_symbol(&self) -> Option<String> {
        self.note_refs_focused_symbol.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    /// Inject a find-notes search backend (a kittest injects a counted in-memory mock so the live
    /// dwell->search->panel pipeline is driven with NO backend — the MT-014/MT-015 fetcher-trait pattern).
    /// The production default is the verified live search-v2 route ([`FindNotesHttp`]).
    pub fn set_find_notes_backend(&self, backend: Arc<dyn FindNotesSearch>) {
        *self.find_notes_backend.lock().unwrap_or_else(|e| e.into_inner()) = backend;
    }

    /// Set the cursor-dwell threshold the live `pump_note_refs` uses (default
    /// [`crate::interop::NOTE_REFS_DWELL_MS`]ms). A kittest sets it to ZERO so the dwell->search->panel
    /// pipeline fires on the first settled frame — driving the REAL wired path deterministically without
    /// an 800ms wall-clock wait. Production never calls this (the 800ms default stands).
    pub fn set_note_refs_dwell_threshold(&self, threshold: std::time::Duration) {
        *self.note_refs_dwell_threshold.lock().unwrap_or_else(|e| e.into_inner()) = threshold;
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
        // Observe the word under the caret this frame; the dwell tracker fires ONCE per dwell crossing.
        let word = self.word_at_primary_cursor();
        let current = if word.is_empty() { None } else { Some(word.as_str()) };
        let threshold = *self.note_refs_dwell_threshold.lock().unwrap_or_else(|e| e.into_inner());
        let fired = {
            let mut dwell = self.note_refs_dwell.lock().unwrap_or_else(|e| e.into_inner());
            dwell.observe_with_threshold(current, std::time::Instant::now(), threshold)
        };
        let Some(dwelled_word) = fired else {
            return; // no dwell crossing this frame -> no search (the debounce suppressed it).
        };

        // IN-FLIGHT GUARD (RISK-3 / MC-3, belt-and-suspenders over the dwell debounce): if a search for
        // the SAME word is already Loading, do NOT stack a second one. The dwell tracker already fires
        // once per crossing, but a same-word re-dwell after the cursor briefly left the word would
        // otherwise stack a redundant search while the first is still in flight — this collapses that to
        // one outstanding search per focused symbol.
        {
            let state = self.note_refs_state.lock().unwrap_or_else(|e| e.into_inner());
            let focused = self.note_refs_focused_symbol.lock().unwrap_or_else(|e| e.into_inner());
            if matches!(*state, NoteRefsState::Loading) && focused.as_deref() == Some(dwelled_word.as_str())
            {
                return;
            }
        }

        // A dwell crossed: mark the panel Loading + record the focused word, then resolve the word to a
        // precise symbol_key and fire the find-notes search off-thread.
        *self.note_refs_state.lock().unwrap_or_else(|e| e.into_inner()) = NoteRefsState::Loading;
        *self.note_refs_focused_symbol.lock().unwrap_or_else(|e| e.into_inner()) =
            Some(dwelled_word.clone());

        let client = self.code_nav_client.clone();
        let backend = Arc::clone(&self.find_notes_backend.lock().unwrap_or_else(|e| e.into_inner()));
        let cell = Arc::clone(&self.note_refs_result);
        let ws = workspace_id.clone();
        runtime.spawn(async move {
            // Resolve the dwelled word to a precise symbol_key (RISK-1: a qualified `path#Symbol` query
            // cuts the false positives a bare word would produce). The lookup is BEST-EFFORT and bounded
            // by a short timeout: if the code-nav backend is slow/unreachable (a headless harness, a flaky
            // server), fall back to the raw word rather than pinning the panel in Loading on a stuck
            // connect (the MT-015 no-perpetual-spinner lesson — the task always completes promptly).
            let lookup = tokio::time::timeout(
                std::time::Duration::from_millis(SYMBOL_KEY_LOOKUP_TIMEOUT_MS),
                client.lookup_symbols(&ws, &dwelled_word, 5),
            )
            .await;
            let symbol_key = match lookup {
                Ok(Ok(syms)) => syms
                    .into_iter()
                    .map(|s| s.symbol_key)
                    .find(|k| !k.trim().is_empty())
                    .unwrap_or_else(|| dwelled_word.clone()),
                // Lookup failed or timed out -> the raw word (the content-type filter still narrows).
                _ => dwelled_word.clone(),
            };
            let state = match find_notes_with(backend.as_ref(), &symbol_key, &ws).await {
                Ok(notes) => NoteRefsState::Loaded(notes),
                Err(e) => NoteRefsState::Failed(e),
            };
            if let Ok(mut slot) = cell.lock() {
                *slot = Some(state);
            }
        });
    }

    /// MT-034: drain a delivered find-notes result into `note_refs_state` (HBR-QUIET — the spawn delivered
    /// it off-thread; here we just swap it in on the UI thread). A no-op when nothing was delivered.
    fn drain_note_refs(&self) {
        if let Some(state) = self.note_refs_result.lock().unwrap_or_else(|e| e.into_inner()).take() {
            *self.note_refs_state.lock().unwrap_or_else(|e| e.into_inner()) = state;
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
            // Route the clicked row through the EXISTING Open-Document cross-pane command (MT-032).
            let bus = InteractionBus::get_or_init(ui.ctx());
            InteractionBus::with_try_lock(&bus, |b| {
                b.register_open_document_command();
                b.open_document(ui.ctx(), doc_id);
            });
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

    /// Redo (Ctrl+Y / Ctrl+Shift+Z) — routed to the host unified-undo stack, same as [`undo`](Self::undo).
    fn redo(&self) {
        self.send_to_command_bus(CodeEditorAction::Redo);
    }

    /// Save (Ctrl+S). Routes the save intent to the host through the command-palette channel as a Save
    /// action (the document shell owns the actual write — the editor does not write files directly). A
    /// no-op + trace when no host channel is wired.
    fn request_save(&self) {
        self.send_to_command_bus(CodeEditorAction::Save);
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
        let tx = self.command_palette_tx.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(tx) = tx.as_ref() {
            let _ = tx.send(action);
        } else {
            tracing::debug!(action = action.name(), "code editor command has no host bus; no-op");
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
                            if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines) {
                                let (line, col) =
                                    self.with_buffer(|b| byte_to_line_col(byte, b));
                                *self.box_drag_start.lock().unwrap_or_else(|e| e.into_inner()) =
                                    Some((line, col));
                            }
                        } else if modifiers.alt {
                            if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines) {
                                self.add_cursor_at(byte);
                            }
                        } else {
                            // Plain click: single caret + clear any box drag.
                            *self.box_drag_start.lock().unwrap_or_else(|e| e.into_inner()) = None;
                            if let Some(byte) = self.pointer_to_byte(*pos, glyph_width, total_lines) {
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
                    ) {
                        continue;
                    }
                    // Skip while a completion popup is open AND the text would be consumed by an accept —
                    // but the popup is non-focus-stealing, so normal typing still flows; only the explicit
                    // Tab/Enter accept (handled in process_keymap) consumes. Insert the text at all
                    // cursors.
                    self.insert_text(text);
                    self.mark_edit_now();
                    if text.chars().any(|c| matches!(c, '.' | ':' | '_')) {
                        self.completion_request.store(true, Ordering::Relaxed);
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
            } else if sig_down {
                self.signature_help_next();
            } else if sig_up {
                self.signature_help_prev();
            }
        }

        // MT-010: ALL key-chord handling is consolidated into the single keymap dispatcher. The
        // per-feature `egui::Event::Key` match arms MT-003/004/005/006/008 each added here are GONE —
        // `process_keymap` resolves every chord through the one `Keymap` table and dispatches the
        // resolved `CodeEditorAction` (including the live-typing Backspace/Delete via DeleteLeft/
        // DeleteRight). Run AFTER the pointer + text handling above so a click-then-key in the same
        // frame sees the updated caret.
        self.process_keymap(ui);

        // WP-KERNEL-012 MT-041 (E7): sync + emit the consolidated `editor.code.<action>` AccessKit nodes
        // and consume any swarm `Action::Click` dispatched at them THIS frame, so a swarm agent's
        // dispatch reaches the editor before the next frame (RISK-041-04). A no-op when no registry is
        // installed (a bare panel render). Run last so it sees the post-input editor state.
        let _dispatched = self.sync_editor_actions(ui);
    }
}

/// A [`PaneFactory`] that mounts a [`CodeEditorPanel`] as a named work-surface pane (MT-001 step 5).
/// Registered for [`PaneType::CodeSymbol`] (the closest existing WP-011 pane variant for a code
/// surface) so the editor appears in the WP-011 docking split layout through the EXISTING pane
/// registry + split layout — no new shell infrastructure is forked.
pub struct CodeEditorPaneFactory {
    panel: Arc<CodeEditorPanel>,
    /// MT-031: set once after the code surface registers its melt-together command set into the shared
    /// bus, so re-registration is idempotent across frames (interior-mutable: the registry borrows
    /// `&dyn PaneFactory` at render time, so `render` has no `&mut self`).
    bus_registered: std::sync::atomic::AtomicBool,
}

impl CodeEditorPaneFactory {
    /// Build a factory wrapping `panel`. `Arc` so the same panel renders across frames without the
    /// factory owning a `&mut` (the registry borrows `&dyn PaneFactory` at render time).
    pub fn new(panel: CodeEditorPanel) -> Self {
        Self { panel: Arc::new(panel), bus_registered: std::sync::atomic::AtomicBool::new(false) }
    }

    /// WP-KERNEL-012 MT-079: build a factory over an EXISTING `Arc<CodeEditorPanel>` so the
    /// session-threading host-mount wrapper (`editor_pane_factories::CodeEditorPaneMount`) and this
    /// inner factory render the SAME panel state. `new` wraps a fresh panel in its own Arc, which would
    /// give the mount and the inner render two different panels; this constructor shares one Arc.
    pub fn from_arc(panel: Arc<CodeEditorPanel>) -> Self {
        Self { panel, bus_registered: std::sync::atomic::AtomicBool::new(false) }
    }

    /// The Arc-shared panel this factory renders (so a test/host can drive the SAME panel state the
    /// mounted pane shows — MT-031 cross-pane proof needs the real panel behind the factory).
    pub fn panel(&self) -> Arc<CodeEditorPanel> {
        Arc::clone(&self.panel)
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
        // The code pane owns the shared selection only while a widget under this pane's egui id scope
        // holds focus (impl note 6/7), so a background pane never clobbers the focused pane's selection.
        // `egui_id` is this pane's scope id; the code editor builds its child widget ids from it.
        let has_focus = ui.memory(|m| {
            m.focused().map(|f| f == ctx.egui_id).unwrap_or(false)
        }) || self.panel.cursors().primary().is_selection();
        let mut registered = self.bus_registered.load(Ordering::Relaxed);
        super::interop_adapter::drive_bus_in_render(
            &bus,
            &self.panel,
            pane_id,
            has_focus,
            &mut registered,
        );
        self.bus_registered.store(registered, Ordering::Relaxed);

        self.panel.show(ui);

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
    }

    fn accesskit_role(&self) -> accesskit::Role {
        accesskit::Role::GenericContainer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn panel_highlights_rust_on_construction() {
        let panel = CodeEditorPanel::new("fn main() { let x = 1; }", "rs");
        assert!(
            panel.spans().iter().any(|s| s.scope == HighlightScope::Keyword),
            "constructed rust panel carries keyword spans"
        );
    }

    #[test]
    fn unknown_extension_panel_has_no_spans_but_renders() {
        let panel = CodeEditorPanel::new("plain text\nsecond line", "txt");
        assert!(panel.spans().is_empty(), "no grammar -> no spans (plain text)");
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
        assert_eq!(stats.buffer_len_lines, 5001, "whole document line count reported");
        // The painted window must be strictly fewer lines than the whole document — that is
        // virtualization. (On a default headless egui Context the CentralPanel viewport is large, so
        // the absolute count depends on viewport height; the load-bearing fact is `painted < total`.
        // The fixed-window kittest screenshot proof asserts the tighter visible-window bound.)
        assert!(
            stats.frame_lines_rendered > 0
                && stats.frame_lines_rendered < stats.buffer_len_lines,
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
        assert_eq!(cached_version, Some(v0 + 1), "cache re-filled at the bumped version");
    }
}
