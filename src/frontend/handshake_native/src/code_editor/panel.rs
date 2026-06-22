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
//! The panel also re-expresses the same arithmetic as a pure [`VirtualLineLayout`] (read back from the
//! persisted `ScrollArea` state) for headless unit-testable boundary math and for [`perf_stats`], the
//! swarm-diagnostics surface that reports how many lines were painted this frame.
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
use super::highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry};
use super::virtual_lines::VirtualLineLayout;

/// The MT-contract author_id for the outer panel container (AC-005: Role::GenericContainer).
pub const CODE_EDITOR_PANEL_AUTHOR_ID: &str = "code_editor_panel";
/// The MT-002 author_id for the virtualized scroll region (AC-004: Role::ScrollView).
pub const CODE_EDITOR_SCROLL_AREA_AUTHOR_ID: &str = "code_editor_scroll_area";
/// The MT-contract author_id for the inner editable text area (AC-005: Role::TextInput).
pub const CODE_EDITOR_TEXT_AUTHOR_ID: &str = "code_editor_text";

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
    /// Number of document lines the row closure painted on the most recent frame (the virtualized
    /// window incl. overscan), or 0 if the panel has not rendered yet.
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
    buffer: TextBuffer,
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
    /// The line index range painted on the most recent frame (the virtualized visible window incl.
    /// overscan), for tests/agents to assert exactly which lines are on screen (AC-003). `0..0` before
    /// the first render.
    last_visible_range: Mutex<std::ops::Range<usize>>,
    /// A one-shot requested vertical scroll offset (px from content top). When set, the next `show`
    /// forces the `ScrollArea` to that offset via `vertical_scroll_offset` and clears the request, so
    /// a caller (a go-to-line action in a later MT, a swarm agent, or a deterministic test) can scroll
    /// the editor to a known position without reaching into egui's persisted scroll state.
    pending_scroll_offset: Mutex<Option<f32>>,
    /// Instance discriminator for AccessKit author_ids (RISK-004). Empty for the default single panel
    /// so it uses the bare MT-contract ids.
    instance: String,
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
            buffer,
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
        }
    }

    /// Borrow the document buffer (for later MTs: cursor/find/fold operate on it).
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
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
        let bytes = self.buffer.to_bytes();
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

    /// The line index range painted on the most recent `show` (the virtualized visible window incl.
    /// overscan). `0..0` before the first render. Lets a test/agent assert exactly which lines are on
    /// screen — the deterministic basis for AC-003 ("line 0 not painted; the scrolled-to region is").
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

        // Highlights are computed at most once per buffer version (cache hit on an unchanged buffer),
        // so the per-frame render never re-parses (MT-002 step 3).
        self.ensure_highlight_cache();

        // Cache the document line count BEFORE the ScrollArea so it is not re-queried inside the row
        // closure (implementation note).
        let total_lines = self.buffer.len_lines();

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
                let scroll_output = scroll_area.show_rows(
                    ui,
                    line_height,
                    total_lines,
                    |ui, row_range| {
                        self.render_rows(ui, row_range, &syntax, total_lines, text_id, &text_author);
                    },
                );

                // Read back the scroll offset egui actually used this frame and re-express the visible
                // window as the pure `VirtualLineLayout` (the documented computation surface). The
                // viewport height is the scroll area's inner rect height. This keeps the headless-test
                // calculator in agreement with the live `show_rows` render and feeds `perf_stats`.
                let viewport_h = scroll_output.inner_rect.height();
                let offset_y = scroll_output.state.offset.y;
                let layout =
                    VirtualLineLayout::new(total_lines, line_height, viewport_h, offset_y);
                let visible = layout.visible_range();
                let painted = visible.len();
                let stats = PerfStats {
                    frame_lines_rendered: painted,
                    buffer_len_lines: total_lines,
                };
                *self.perf.lock().unwrap_or_else(|e| e.into_inner()) = stats;
                *self.last_visible_range.lock().unwrap_or_else(|e| e.into_inner()) = visible;

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

    /// Render the rows for `row_range` (the virtualized visible window `show_rows` selected) and emit
    /// the inner `Role::TextInput` node. Split out so the text-area scope nests under the scroll-area
    /// scope (parent->child linkage for AC-005). The node is emitted onto this nested scope's own `Ui`
    /// id, which egui parents under the scroll scope's `Ui` node.
    fn render_rows(
        &self,
        ui: &mut egui::Ui,
        row_range: std::ops::Range<usize>,
        syntax: &HsSyntaxTokens,
        total_lines: usize,
        text_id: egui::Id,
        text_author: &str,
    ) {
        ui.scope_builder(egui::UiBuilder::new().id_salt(text_id), |ui| {
            let text_node_id = ui.unique_id();
            ui.style_mut().spacing.item_spacing.y = 0.0;

            // Paint ONLY the lines `show_rows` selected (the visible window + egui's overscan). Clamp
            // the upper bound to the document length defensively (show_rows already clamps, but a
            // stale range must never index past the buffer).
            let end = row_range.end.min(total_lines);

            // CLIP the highlight span list to the visible BYTE window ONCE per frame (MT-002 step 3),
            // rather than scanning the whole span list per line. On a 100k-line file the cache holds
            // hundreds of thousands of spans; an O(visible_lines * all_spans) per-line scan is the
            // dominant frame cost. The cache is sorted by start byte, so a binary search bounds the
            // window to just the spans that can touch the painted rows.
            let win_start = self.buffer.line_to_byte(row_range.start).unwrap_or(0);
            let win_end = self
                .buffer
                .line_to_byte(end)
                .unwrap_or_else(|| self.buffer.len_bytes());
            let visible_spans = self.spans_in_byte_window(win_start, win_end);

            for line_idx in row_range.start..end {
                self.render_line(ui, line_idx, &visible_spans, syntax);
            }

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
        let line_text = self.buffer.slice_to_string(line_idx..line_idx + 1);
        // Strip the trailing newline so each visual line is one row (the layout adds the row break).
        let line_text = line_text.strip_suffix('\n').unwrap_or(&line_text);
        let line_start_byte = self.buffer.line_to_byte(line_idx).unwrap_or(0);
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
