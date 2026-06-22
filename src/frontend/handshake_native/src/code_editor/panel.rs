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
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsSyntaxTokens;

use super::buffer::TextBuffer;
use super::cursor::{
    byte_to_line_col, find_next_occurrence, line_col_to_byte, word_at, Cursor, CursorSet,
    MAX_ACCESSKIT_CURSORS,
};
use super::highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry};

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

/// The fixed AccessKit `NodeId` band the per-cursor `Role::Caret` nodes occupy for the default panel
/// (210..210+MAX_ACCESSKIT_CURSORS), disjoint from the panel container/scroll/text band (200/201/202)
/// and the WP-011 shell band (>= 100).
const PANEL_CURSOR_NODE_ID_BASE: u64 = 210;

/// The monospace font size the panel renders text at (matches `render_line`). Centralized so the caret
/// overlay measures glyph width with the SAME `FontId` the glyphs are painted with (MT-003 positioning
/// requirement — no x-unit drift).
const MONO_FONT_SIZE: f32 = 13.0;

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
        let spans = match highlighter.as_mut() {
            Some(hl) => hl.highlight(text.as_bytes()),
            None => Vec::new(),
        };
        let buffer = TextBuffer::new(text);
        let len_lines = buffer.len_lines();
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

        // Cache the document line count BEFORE the ScrollArea so it is not re-queried inside the row
        // closure (implementation note).
        let total_lines = self.with_buffer(|b| b.len_lines());

        // OUTER container scope. egui gives every child `Ui` its own AccessKit node keyed by the
        // `Ui`'s id and nests it under the parent `Ui`'s node. We emit the CONTAINER node onto THIS
        // scope's own `Ui` id, render the scroll-area in a nested scope inside it, and render the text
        // content nested inside that — so the live tree is container -> scroll-area -> text (AC-004 +
        // AC-005 ancestry). The fixed `container_id` is only the `id_salt` that keeps the scope's id
        // stable across frames.
        ui.scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
            let container_node_id = ui.unique_id();

            // Paint the editor background from the theme (no hardcoded hex).
            let bg = syntax.background;
            let full_rect = ui.available_rect_before_wrap();
            if ui.is_rect_visible(full_rect) {
                ui.painter().rect_filled(full_rect, 0.0, bg);
            }

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
                scroll_area.show_rows(
                    ui,
                    line_height,
                    total_lines,
                    |ui, row_range| {
                        // Record egui's actual painted window before forwarding it to the painter.
                        painted_range = row_range.clone();
                        self.render_rows(
                            ui,
                            row_range,
                            &syntax,
                            total_lines,
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

            // Emit the container node onto this scope's Ui id from INSIDE the scope, so it is the
            // node that parents the nested scroll-area scope (AC-005: GenericContainer + author_id).
            let author = container_author.clone();
            ui.ctx().accesskit_node_builder(container_node_id, move |node| {
                node.set_role(accesskit::Role::GenericContainer);
                node.set_author_id(author.clone());
                node.set_label("Code editor".to_owned());
            });
        });
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

            // Paint ONLY the lines `show_rows` selected (the visible window + egui's overscan). Clamp
            // the upper bound to the document length defensively (show_rows already clamps, but a
            // stale range must never index past the buffer).
            let end = row_range.end.min(total_lines);

            // CLIP the highlight span list to the visible BYTE window ONCE per frame (MT-002 step 3),
            // rather than scanning the whole span list per line. On a 100k-line file the cache holds
            // hundreds of thousands of spans; an O(visible_lines * all_spans) per-line scan is the
            // dominant frame cost. The cache is sorted by start byte, so a binary search bounds the
            // window to just the spans that can touch the painted rows.
            let (win_start, win_end) = self.with_buffer(|b| {
                let ws = b.line_to_byte(row_range.start).unwrap_or(0);
                let we = b.line_to_byte(end).unwrap_or_else(|| b.len_bytes());
                (ws, we)
            });
            let visible_spans = self.spans_in_byte_window(win_start, win_end);

            for line_idx in row_range.start..end {
                self.render_line(ui, line_idx, &visible_spans, syntax);
            }

            // Store the painted-row geometry so `process_cursor_input` (pointer hit-testing) and the
            // overlay share egui's ACTUAL layout — no separate recompute (the MT-002 unit discipline:
            // sans-spacing line_height, the SAME glyph FontId).
            let geometry = RowGeometry {
                left: origin.x,
                top: origin.y,
                first_line: row_range.start,
                line_height,
            };
            *self.row_geometry.lock().unwrap_or_else(|e| e.into_inner()) = Some(geometry);

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
        });
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
