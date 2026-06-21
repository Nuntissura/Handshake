//! egui widget that renders the native code editor panel (WP-KERNEL-012 MT-001).
//!
//! [`CodeEditorPanel`] owns a [`TextBuffer`] + a [`Highlighter`] and paints the visible lines with
//! per-scope theme colors. It exposes two stable AccessKit nodes a swarm agent addresses:
//! - an OUTER `Role::GenericContainer` node with `author_id = "code_editor_panel"` (the panel frame),
//! - an INNER `Role::TextInput` node with `author_id = "code_editor_text"` (the editable text area),
//!   emitted INSIDE the container's egui scope so the live AccessKit tree links them parent->child
//!   (the same nesting linkage the WP-011 shell relies on).
//!
//! ## Theme-driven colors (no hardcoded hex)
//!
//! [`scope_to_color`] maps each [`HighlightScope`] to a color taken from the active theme's
//! [`HsSyntaxTokens`] (`theme/syntax.rs`). The panel reads the live `egui::Visuals` to decide
//! dark/light and pulls the matching token set, so it never embeds a `Color32` literal (the
//! no-hardcode invariant the theme layer enforces).
//!
//! ## Render cap before virtualization (RISK-003)
//!
//! Until MT-002 lands viewport virtualization, [`CodeEditorPanel::show`] renders at most
//! [`MAX_RENDERED_LINES`] (1000) lines, slicing only that window out of the rope (never
//! `.to_string()`-ing the whole document every frame). A >10k-line file therefore cannot block the
//! egui frame at startup.
//!
//! ## author_id instance suffix (RISK-004)
//!
//! Multiple panels (e.g. a diff view mounting two editors) would collide on the fixed author_ids.
//! Each [`CodeEditorPanel`] carries an `instance` string; [`CodeEditorPanel::with_instance`] appends
//! it (`code_editor_panel#<instance>`) so concurrently-mounted panels stay individually addressable.
//! The default (single) panel uses the bare ids the MT contract names so AC-005 matches exactly.

use std::sync::Arc;

use egui::accesskit;

use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsSyntaxTokens;

use super::buffer::TextBuffer;
use super::highlight::{HighlightScope, HighlightSpan, Highlighter, LanguageRegistry};

/// The MT-contract author_id for the outer panel container (AC-005: Role::GenericContainer).
pub const CODE_EDITOR_PANEL_AUTHOR_ID: &str = "code_editor_panel";
/// The MT-contract author_id for the inner editable text area (AC-005: Role::TextInput).
pub const CODE_EDITOR_TEXT_AUTHOR_ID: &str = "code_editor_text";

/// Max lines rendered per frame before MT-002 virtualization (RISK-003 guard). A document longer
/// than this still loads into the buffer fully; only the rendered window is capped so the frame
/// stays bounded.
pub const MAX_RENDERED_LINES: usize = 1000;

/// Fixed AccessKit `NodeId`s for the default (single-instance) panel. They sit in a fresh band
/// (200/201) ABOVE the WP-011 pane id space (>= 100) so they cannot collide with shell chrome,
/// dividers, or panes. Multi-instance panels (RISK-004) derive their ids by hashing the suffixed
/// author_id into egui's hashed id space instead of this fixed band.
const PANEL_CONTAINER_NODE_ID: u64 = 200;
const PANEL_TEXT_NODE_ID: u64 = 201;

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
/// visible lines as colored runs.
pub struct CodeEditorPanel {
    buffer: TextBuffer,
    /// `None` when the document's extension has no registered grammar (plain text, no highlighting).
    highlighter: Option<Highlighter>,
    /// Cached spans from the last highlight pass; recomputed when the buffer changes (a generation
    /// counter would be added in MT-002+; for now the panel recomputes on demand via `refresh`).
    spans: Vec<HighlightSpan>,
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
        Self {
            buffer: TextBuffer::new(text),
            highlighter,
            spans,
            instance,
        }
    }

    /// Borrow the document buffer (for later MTs: cursor/find/fold operate on it).
    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    /// The current highlight spans (read by tests + later MTs' minimap/outline).
    pub fn spans(&self) -> &[HighlightSpan] {
        &self.spans
    }

    /// Re-run highlighting over the current buffer (called after an edit; MT-002+ will gate this on a
    /// dirty flag). No-op highlighter -> empty spans.
    pub fn refresh(&mut self) {
        let bytes = self.buffer.to_bytes();
        self.spans = match self.highlighter.as_mut() {
            Some(hl) => hl.highlight(&bytes),
            None => Vec::new(),
        };
    }

    /// The stable AccessKit author_id for this panel's outer container, with the instance suffix when
    /// present (RISK-004).
    pub fn container_author_id(&self) -> String {
        if self.instance.is_empty() {
            CODE_EDITOR_PANEL_AUTHOR_ID.to_owned()
        } else {
            format!("{CODE_EDITOR_PANEL_AUTHOR_ID}#{}", self.instance)
        }
    }

    /// The stable AccessKit author_id for this panel's inner text area, with the instance suffix when
    /// present (RISK-004).
    pub fn text_author_id(&self) -> String {
        if self.instance.is_empty() {
            CODE_EDITOR_TEXT_AUTHOR_ID.to_owned()
        } else {
            format!("{CODE_EDITOR_TEXT_AUTHOR_ID}#{}", self.instance)
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

    /// The fixed `egui::Id` for the inner text area (band slot 201 for the default panel; hashed for
    /// instances). See [`container_id`](Self::container_id) for the safety rationale.
    fn text_id(&self) -> egui::Id {
        if self.instance.is_empty() {
            unsafe { egui::Id::from_high_entropy_bits(PANEL_TEXT_NODE_ID) }
        } else {
            egui::Id::new(self.text_author_id())
        }
    }

    /// Render the panel into `ui`: a scrollable, theme-colored view of the buffer's visible lines plus
    /// the two AccessKit nodes (container + text). Renders at most [`MAX_RENDERED_LINES`] lines
    /// (RISK-003). Safe to call every frame.
    pub fn show(&self, ui: &mut egui::Ui) {
        let syntax = syntax_tokens_for(ui.visuals());
        let container_id = self.container_id();
        let container_author = self.container_author_id();
        let text_author = self.text_author_id();
        let text_id = self.text_id();

        // OUTER container scope. egui gives every child `Ui` its own AccessKit node keyed by the
        // `Ui`'s id, and nests it under the parent `Ui`'s node. We emit the CONTAINER node onto THIS
        // scope's own `Ui` id (`ui.id()`), and render the text content in a nested scope inside it, so
        // the text node is a genuine DESCENDANT of the container node in the live tree (AC-005
        // ancestry), not a sibling. The fixed `container_id` is only the `id_salt` that keeps the
        // scope's id stable across frames.
        ui.scope_builder(egui::UiBuilder::new().id_salt(container_id), |ui| {
            // egui auto-creates a GenericContainer AccessKit node for every child `Ui`, keyed by the
            // Ui's `unique_id`, AND registers that node's parent in the live accessibility tree (a
            // child `Ui`'s parent is the enclosing `Ui`'s `unique_id`). We OVERWRITE this scope's own
            // node with the editor container's author_id/label, and likewise overwrite the nested text
            // scope's node, so the text node is a genuine DESCENDANT of the container node via egui's
            // already-registered parent chain (AC-005 ancestry) — emitting onto `ui.id()` (the salted
            // display id) instead would NOT be parented (egui keys the parent map by `unique_id`).
            let container_node_id = ui.unique_id();

            // Paint the editor background from the theme (no hardcoded hex).
            let bg = syntax.background;
            let full_rect = ui.available_rect_before_wrap();
            if ui.is_rect_visible(full_rect) {
                ui.painter().rect_filled(full_rect, 0.0, bg);
            }

            egui::ScrollArea::vertical()
                .id_salt(("code-editor-scroll", container_id))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.render_text_area(ui, &syntax, text_id, &text_author);
                });

            // Emit the container node onto this scope's Ui id from INSIDE the scope, so it is the
            // node that parents the nested text scope (AC-005: GenericContainer + author_id).
            let author = container_author.clone();
            ui.ctx().accesskit_node_builder(container_node_id, move |node| {
                node.set_role(accesskit::Role::GenericContainer);
                node.set_author_id(author.clone());
                node.set_label("Code editor".to_owned());
            });
        });
    }

    /// Render the highlighted text lines and emit the inner `Role::TextInput` node. Split out so the
    /// text-area scope nests under the container scope (parent->child linkage for AC-005). The node is
    /// emitted onto this nested scope's own `Ui` id, which egui parents under the container scope's
    /// `Ui` node — making the text node a descendant of the container node in the live tree.
    fn render_text_area(
        &self,
        ui: &mut egui::Ui,
        syntax: &HsSyntaxTokens,
        text_id: egui::Id,
        text_author: &str,
    ) {
        ui.scope_builder(egui::UiBuilder::new().id_salt(text_id), |ui| {
            // Overwrite THIS scope's auto-created node (keyed by its `unique_id`) with the text-area
            // author_id/role. egui has already registered this scope's parent as the enclosing Ui, so
            // the node lands under the container node in the live tree (AC-005 ancestry).
            let text_node_id = ui.unique_id();
            ui.style_mut().spacing.item_spacing.y = 0.0;
            let total_lines = self.buffer.len_lines();
            let rendered = total_lines.min(MAX_RENDERED_LINES);

            for line_idx in 0..rendered {
                self.render_line(ui, line_idx, syntax);
            }

            if total_lines > MAX_RENDERED_LINES {
                // Honest, theme-colored notice that the render is capped pending MT-002 virtualization
                // (RISK-003) — not a silent truncation.
                ui.colored_label(
                    syntax.comment,
                    format!(
                        "… {} more lines (virtualization lands in MT-002)",
                        total_lines - MAX_RENDERED_LINES
                    ),
                );
            }

            // Emit the TextInput node onto this nested scope's Ui id (AC-005). Because this scope is a
            // child of the container scope, the node is a descendant of the container node.
            let line_count = self.buffer.len_lines();
            let author = text_author.to_owned();
            ui.ctx().accesskit_node_builder(text_node_id, move |node| {
                node.set_role(accesskit::Role::TextInput);
                node.set_author_id(author.clone());
                node.set_label("Code editor text".to_owned());
                node.set_value(format!("{line_count} lines"));
            });
        });
    }

    /// Render one line as a sequence of theme-colored runs, splitting the line text at the highlight
    /// span boundaries that overlap it. A line with no overlapping spans renders as plain foreground
    /// text. Byte->char conversions go through the buffer (RISK-002).
    fn render_line(&self, ui: &mut egui::Ui, line_idx: usize, syntax: &HsSyntaxTokens) {
        let line_text = self.buffer.slice_to_string(line_idx..line_idx + 1);
        // Strip the trailing newline so each visual line is one row (the layout adds the row break).
        let line_text = line_text.strip_suffix('\n').unwrap_or(&line_text);
        let line_start_byte = self.buffer.line_to_byte(line_idx).unwrap_or(0);
        let line_end_byte = line_start_byte + line_text.len();

        // Spans overlapping this line, clipped to the line's byte window.
        let mut runs: Vec<(std::ops::Range<usize>, HighlightScope)> = Vec::new();
        for span in &self.spans {
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
            // convert through the buffer's char-aware API; fall back to empty on a bad boundary).
            let line_slice = |start: usize, end: usize| -> String {
                let rel_start = start.saturating_sub(line_start_byte);
                let rel_end = end.saturating_sub(line_start_byte);
                if rel_start >= rel_end || rel_end > line_text.len() {
                    return String::new();
                }
                // Respect char boundaries: walk to the nearest valid boundary if needed.
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
        assert_eq!(b.container_author_id(), "code_editor_panel#right");
        assert_ne!(a.container_author_id(), b.container_author_id());
        assert_ne!(a.text_author_id(), b.text_author_id());
        // The default panel uses the bare MT-contract ids (AC-005).
        let d = CodeEditorPanel::new("z", "rs");
        assert_eq!(d.container_author_id(), CODE_EDITOR_PANEL_AUTHOR_ID);
        assert_eq!(d.text_author_id(), CODE_EDITOR_TEXT_AUTHOR_ID);
    }

    #[test]
    fn large_document_render_is_capped() {
        // 5000 lines -> the panel must render at most MAX_RENDERED_LINES without blocking (RISK-003).
        let big = "x\n".repeat(5000);
        let panel = CodeEditorPanel::new(&big, "rs");
        assert!(panel.buffer().len_lines() > MAX_RENDERED_LINES);
        let ctx = egui::Context::default();
        // Just proving it runs a frame without panic / unbounded work.
        let _ = ctx.run(Default::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| panel.show(ui));
        });
    }
}
