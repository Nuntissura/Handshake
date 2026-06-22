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

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsSyntaxTokens;

use super::buffer::TextBuffer;
use super::code_nav::{
    staleness_marker_for, CodeNavCache, CodeNavClient, CodeSymbolNavProjection, CompletionItem,
    COMPLETION_DEBOUNCE_MS, HOVER_DWELL_MS, SYMBOL_LOOKUP_LIMIT,
};
use super::cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet,
    MAX_ACCESSKIT_CURSORS,
};
use super::editor_view::{
    CompletionOutcome, CompletionPopup, CompletionState, HoverOutcome, HoverState, HoverTooltip,
};
use super::lsp_client::{LspClient, PublishedDiagnostics};
use super::find_replace::{FindEngine, FindQuery, Match};
use super::breakpoints::{BreakpointAction, BreakpointEvent, BreakpointSet};
use super::folding::{FoldProvider, FoldSet};
use super::gutter::{
    DiagnosticSeverity, Gutter, GutterConfig, GutterGeometry, GutterMarker, GutterMarkerKind,
    GutterResponse,
};
use super::highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry};
use super::minimap::Minimap;
use super::outline::{OutlineItem, OutlineProvider};

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
    /// MT-008 the LSP `publishDiagnostics` receiver, parked on the panel after it is taken (once) from
    /// the LSP client, so [`drain_lsp_diagnostics`](Self::drain_lsp_diagnostics) can incrementally drain
    /// it each frame and route notifications to the gutter (AC-008). `None` until the first drain takes
    /// the receiver from a configured client.
    lsp_diagnostics_rx:
        Mutex<Option<tokio::sync::mpsc::UnboundedReceiver<PublishedDiagnostics>>>,
}

/// MT-008 off-thread completion result delivery cell: `(cursor anchor pixel, popup items)` written by
/// a spawned `lookup_symbols` task and drained on the next frame. Aliased so the panel field type stays
/// legible (clippy `type_complexity`).
type CompletionResultCell = Arc<Mutex<Option<(egui::Pos2, Vec<CompletionItem>)>>>;

/// MT-008 off-thread hover result delivery cell: `(cursor anchor pixel, hover state)` written by a
/// spawned hover task and drained on the next frame. Aliased for the same legibility reason.
type HoverResultCell = Arc<Mutex<Option<(egui::Pos2, HoverState)>>>;

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
            // The outline was computed at buffer version 1 (same as folds/highlights), so the first
            // access is a cache hit; an edit bumps the version and recomputes from the new tree.
            outline_items: Mutex::new(outline_items),
            outline_version: AtomicU64::new(1),
            show_outline: std::sync::atomic::AtomicBool::new(outline_default_on),
            show_minimap: std::sync::atomic::AtomicBool::new(true),
            goto_line_state: Mutex::new(None),
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
            lsp_diagnostics_rx: Mutex::new(None),
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
                self.navigate_to_line(line);
                self.close_goto_line();
                true
            }
            None => false, // invalid input: no navigation, palette stays open (AC-002).
        }
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

        // Measure + cache the monospace line height once (implementation note: do it at first show
        // and reuse). `show_rows` needs the per-line height WITHOUT egui's row spacing (it adds the
        // spacing itself), and we zero item-spacing inside the rows, so the measured glyph height is
        // the row height.
        let line_height = self.line_height(ui);
        // Measure + cache the monospace glyph width once, with the SAME FontId render_line paints with,
        // so the caret/selection overlay (MT-003) aligns column->x exactly (implementation note 4).
        let glyph_width = self.glyph_width(ui);

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
                // MT-005: drive `show_rows` over the VISIBLE (post-fold) line count, so a folded region
                // collapses the scroll content (the scrollbar reflects the folded document). The
                // `row_range` egui hands the closure is therefore in VISIBLE-line space; `render_rows`
                // maps each visible row back to a buffer line via the FoldSet (MT step 4/6).
                scroll_area.show_rows(
                    ui,
                    line_height,
                    visible_lines,
                    |ui, row_range| {
                        // Record egui's actual painted window (visible-line space) before painting.
                        painted_range = row_range.clone();
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
                    self.navigate_to_line(line);
                    self.close_hover();
                }
                HoverOutcome::None => {}
            }
        }
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

            // MT-004: paint the find-match highlights FIRST (below the carets) so a caret/selection
            // stays visible on top of a match rect. Restricted to the painted row window (the same
            // sans-spacing line_height + monospace glyph_width units as the cursor overlay).
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

    /// Process this frame's egui input for the multi-cursor bindings (MT-003 steps 2-5). Reads pointer
    /// + key events from `ui`'s context:
    /// - Alt+Click -> add a caret at the clicked position; plain Primary click -> single caret.
    /// - Ctrl+Alt+Up / Ctrl+Alt+Down -> add a caret above / below each cursor.
    /// - Ctrl+D -> select the word / add the next occurrence.
    /// - Alt+Shift drag -> box/column selection across the dragged line/column range.
    ///
    /// The bindings mirror Monaco/VS Code exactly. Input is only consumed when the pointer is over the
    /// editor rows (so a click elsewhere in the shell does not move the editor caret).
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
                // KEYS: Ctrl+Alt+Up/Down add cursor above/below; Ctrl+D selects the next occurrence.
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => match key {
                    egui::Key::ArrowUp if modifiers.ctrl && modifiers.alt => {
                        self.add_cursor_above();
                    }
                    egui::Key::ArrowDown if modifiers.ctrl && modifiers.alt => {
                        self.add_cursor_below();
                    }
                    egui::Key::D if modifiers.ctrl && !modifiers.alt && !modifiers.shift => {
                        self.select_next_occurrence();
                    }
                    // MT-005 step 5: Ctrl+Shift+[ folds the region at the cursor; Ctrl+Shift+] unfolds.
                    // (MT-010 will formalize the full keymap; pre-wired here.) Intercepted regardless of
                    // pointer position so the fold keymap works whenever the editor has focus.
                    egui::Key::OpenBracket if modifiers.ctrl && modifiers.shift && !modifiers.alt => {
                        self.fold_at_cursor();
                    }
                    egui::Key::CloseBracket if modifiers.ctrl && modifiers.shift && !modifiers.alt => {
                        self.unfold_at_cursor();
                    }
                    // MT-006 step 3: Ctrl+G opens the go-to-line palette. While it is open, Enter submits
                    // (parse + fold-aware navigate) and Escape closes. These are checked BEFORE the find
                    // Enter/Escape arms so the go-to-line palette takes precedence when both could match
                    // (the palette is the focused modal). Intercepted regardless of pointer position.
                    egui::Key::G if modifiers.ctrl && !modifiers.alt && !modifiers.shift => {
                        self.open_goto_line();
                    }
                    egui::Key::Escape if self.is_goto_line_open() => {
                        self.close_goto_line();
                    }
                    egui::Key::Enter if self.is_goto_line_open() => {
                        // Submit: valid numeric -> navigate + close; invalid -> stays open (AC-002).
                        self.submit_goto_line();
                    }
                    // MT-004 step 2: Ctrl+F opens find; Ctrl+H opens find with the replace row; Escape
                    // (when the bar is open) closes it. Enter / Shift+Enter step to the next / prev match
                    // (Monaco/VS Code parity). These are intercepted regardless of pointer position so
                    // the keymap works whether or not the editor area has the pointer over it.
                    egui::Key::F if modifiers.ctrl && !modifiers.alt && !modifiers.shift => {
                        self.open_find(false);
                    }
                    egui::Key::H if modifiers.ctrl && !modifiers.alt && !modifiers.shift => {
                        self.open_find(true);
                    }
                    egui::Key::Escape if self.is_find_open() => {
                        self.close_find();
                    }
                    egui::Key::Enter if self.is_find_open() && !modifiers.shift => {
                        self.next_match();
                    }
                    egui::Key::Enter if self.is_find_open() && modifiers.shift => {
                        self.prev_match();
                    }
                    _ => {}
                },
                _ => {}
            }
        }
    }
}

/// A [`PaneFactory`] that mounts a [`CodeEditorPanel`] as a named work-surface pane (MT-001 step 5).
/// Registered for [`PaneType::CodeSymbol`] (the closest existing WP-011 pane variant for a code
/// surface) so the editor appears in the WP-011 docking split layout through the EXISTING pane
/// registry + split layout — no new shell infrastructure is forked.
pub struct CodeEditorPaneFactory {
    panel: Arc<CodeEditorPanel>,
}

impl CodeEditorPaneFactory {
    /// Build a factory wrapping `panel`. `Arc` so the same panel renders across frames without the
    /// factory owning a `&mut` (the registry borrows `&dyn PaneFactory` at render time).
    pub fn new(panel: CodeEditorPanel) -> Self {
        Self { panel: Arc::new(panel) }
    }
}

impl PaneFactory for CodeEditorPaneFactory {
    fn pane_type(&self) -> PaneType {
        PaneType::CodeSymbol
    }

    fn render(&self, ui: &mut egui::Ui, _ctx: &PaneRenderContext) {
        self.panel.show(ui);
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
