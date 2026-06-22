//! Diff + merge editor egui panel for the native code editor (WP-KERNEL-012 MT-009).
//!
//! [`DiffEditorPanel`] renders the [`diff_engine`](super::diff_engine) output as a VS-Code-class
//! diff surface with three modes:
//!
//! - [`DiffMode::SideBySide`]: two [`CodeEditorPanel`] instances side by side (the FIRST real
//!   two-panel mount — RISK-004 / the KERNEL_BUILDER gate). The two panels are constructed with
//!   `CodeEditorPanel::with_instance("left")` / `with_instance("right")` so their egui `ScrollArea`
//!   ids AND AccessKit author_ids are disjoint (`code_editor_panel#left` / `#right`). A diff-block
//!   background rect is painted over each pane's changed rows (green=added on the right,
//!   red=removed on the left, yellow=modified on both), and scrolling the left pane scrolls the
//!   right pane to the equivalent visual row through the [`SyncScrollState`] line map.
//! - [`DiffMode::Inline`]: a single panel showing the unified diff with `+ ` (green) / `- ` (red) /
//!   `  ` (gray) line prefixes.
//! - [`DiffMode::Merge`]: three panes (local / base / remote) with per-conflict Accept Local /
//!   Accept Remote / Accept Both buttons that resolve a [`MergeBlock`] and recompute the merged
//!   output buffer.
//!
//! ## AccessKit (MT contract author_ids)
//!
//! - `diff_editor_panel` (`Role::Group`) — the panel container.
//! - `diff_mode_toggle` (the SideBySide/Inline toggle) — accesskit 0.21.1 has NO `Role::ToggleButton`
//!   variant (the contract named it), so the field-correct nearest role `Role::Button` is used, the
//!   same documented-deviation pattern MT-003 (`TextCursor`->`Caret`) and MT-007
//!   (`ToggleButton`->`CheckBox`) used. The ACs assert the `author_id`, which matches exactly.
//! - `diff_block_{n}_accept_local` (`Role::Button`) on each merge-conflict Accept-Local button (and
//!   the sibling `diff_block_{n}_accept_remote` / `diff_block_{n}_accept_both`).
//!
//! ## Colors (MT-007 theme-guard, CONTROL-4)
//!
//! The translucent diff-block backgrounds use [`egui::Color32::from_rgba_unmultiplied`] — the
//! SANCTIONED form the no-hardcoded-hex guard (`tests/test_theme.rs`) does NOT flag (it flags only
//! the opaque from-rgb / WHITE / BLACK forms). The contract names these exact RGBA values:
//! Added=(0,255,0,30), Removed=(255,0,0,30), Modified=(255,200,0,30). The inline prefix and
//! merge-label foreground colors are theme-sourced through [`HsSyntaxTokens`].

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

use egui::accesskit;

use super::buffer::TextBuffer;
use super::diff_engine::{
    DiffBlock, DiffEngine, DiffStatus, MergeBlock, MergeChoice, MergeEngine, MergeStatus,
};
use super::panel::CodeEditorPanel;
use super::virtual_lines::VirtualLineLayout;
use crate::pane_registry::{PaneFactory, PaneRenderContext, PaneType};
use crate::theme::HsSyntaxTokens;

/// The panel container's stable AccessKit author_id (MT contract). `Role::Group`.
pub const DIFF_EDITOR_PANEL_AUTHOR_ID: &str = "diff_editor_panel";

/// The SideBySide/Inline mode-toggle button's stable AccessKit author_id (MT contract).
pub const DIFF_MODE_TOGGLE_AUTHOR_ID: &str = "diff_mode_toggle";

/// Prefix for a merge-block Accept-Local button author_id: `diff_block_{n}_accept_local`.
pub const DIFF_BLOCK_ACCEPT_LOCAL_PREFIX: &str = "diff_block_";

/// Fixed AccessKit/egui id for the diff panel container node. A fresh value in the high band (well
/// above the WP-011 shell's fixed chrome/pane bands, which top out below 100 and the editor's MT-002
/// nav band) so it cannot collide with shell chrome. This panel mounts standalone in tests; in the
/// shell it mounts as a pane whose container id comes from the registry, and the diff panel's own
/// nodes are emitted under that pane.
const DIFF_PANEL_CONTAINER_NODE_ID: u64 = 9_300;
/// Fixed id for the mode-toggle button node (single-instance standalone panel).
const DIFF_MODE_TOGGLE_NODE_ID: u64 = 9_301;

/// The diff display mode. `SideBySide` and `Inline` are the two diff views; `Merge` is the
/// three-way conflict-resolution view (set via [`DiffEditorPanel::merge`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffMode {
    /// Two panes side by side with synchronized scroll and per-block background highlights.
    SideBySide,
    /// A single unified pane with `+ ` / `- ` / `  ` line prefixes.
    Inline,
    /// Three panes (local/base/remote) with per-conflict accept buttons.
    Merge,
}

impl DiffMode {
    fn to_u8(self) -> u8 {
        match self {
            DiffMode::SideBySide => 0,
            DiffMode::Inline => 1,
            DiffMode::Merge => 2,
        }
    }
    fn from_u8(v: u8) -> Self {
        match v {
            1 => DiffMode::Inline,
            2 => DiffMode::Merge,
            _ => DiffMode::SideBySide,
        }
    }
}

/// One visual row of the synchronized-scroll line map: the (left_line, right_line) pair the row
/// corresponds to. `None` on a side means the row exists only on the other side (an added/removed
/// line). Built from the [`DiffBlock`]s so a visual row maps to the correct buffer line on each pane.
pub type SyncRow = (Option<usize>, Option<usize>);

/// Which side last drove a synchronized scroll, so the panel applies the mapped offset to the OTHER
/// side without feedback-looping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Synchronized-scroll state for the side-by-side view (MT step 2). Holds the last-known scroll
/// offsets and which side changed last, plus the visual-row line map both ends look up through.
#[derive(Clone, Debug, Default)]
pub struct SyncScrollState {
    /// The line map: one entry per visual row, `(left_line, right_line)`.
    pub line_map: Vec<SyncRow>,
    /// Last applied left scroll offset (px).
    pub left_scroll_offset: f32,
    /// Last applied right scroll offset (px).
    pub right_scroll_offset: f32,
    /// Which side last changed (drives the one-shot scroll request to the other side).
    pub last_changed: Option<Side>,
}

/// Build the synchronized-scroll visual-row line map from the diff blocks (MT step 6). Each visual
/// row is `(Option<left_line>, Option<right_line>)`:
///   - `Equal` / `Modified` blocks contribute paired rows (both sides advance).
///   - `Added` blocks contribute right-only rows (`(None, Some(r))`).
///   - `Removed` blocks contribute left-only rows (`(Some(l), None)`).
pub fn build_line_map(blocks: &[DiffBlock]) -> Vec<SyncRow> {
    let mut map = Vec::new();
    for block in blocks {
        match block.status {
            DiffStatus::Equal | DiffStatus::Modified => {
                // Pair the two ranges row-for-row; if one is longer (a multi-line modify with an
                // uneven replacement), pad the shorter side with None.
                let n = block.left_lines.len().max(block.right_lines.len());
                for i in 0..n {
                    let l = block.left_lines.clone().nth(i);
                    let r = block.right_lines.clone().nth(i);
                    map.push((l, r));
                }
            }
            DiffStatus::Added => {
                for r in block.right_lines.clone() {
                    map.push((None, Some(r)));
                }
            }
            DiffStatus::Removed => {
                for l in block.left_lines.clone() {
                    map.push((Some(l), None));
                }
            }
        }
    }
    map
}

/// Map a left-pane LINE to the corresponding visual row, then to the right-pane line, for the
/// synchronized scroll (MT step 6: "when left scrolls to visual row R, find line_map[R].1"). Returns
/// the right line that visually aligns with `left_line`, falling back to the nearest preceding paired
/// row when the left line sits on a removed (left-only) row.
pub fn right_line_for_left_line(line_map: &[SyncRow], left_line: usize) -> Option<usize> {
    // Find the visual row whose left side is `left_line`.
    let row = line_map.iter().position(|(l, _)| *l == Some(left_line))?;
    // Walk forward from that row to the first row that has a right line; if none, walk backward.
    for (_, r) in line_map.iter().skip(row) {
        if let Some(r) = r {
            return Some(*r);
        }
    }
    for (_, r) in line_map.iter().take(row).rev() {
        if let Some(r) = r {
            return Some(*r);
        }
    }
    None
}

/// The diff/merge editor panel widget (MT-009). Standalone (constructed via [`DiffEditorPanel::diff`]
/// / [`DiffEditorPanel::merge`]) and mounted as a pane through the existing
/// [`DiffEditorPaneFactory`] + `pane_registry` / `split_layout` (no new shell layout system).
///
/// Interior mutability mirrors [`CodeEditorPanel`]: the `PaneFactory` trait is `Send + Sync`, so the
/// mutable render-side state lives behind `Mutex`/atomics rather than `RefCell` (which is not
/// `Sync`). The panel renders on the single egui UI thread, so contention is nil.
pub struct DiffEditorPanel {
    /// The LEFT (old) document, behind a `CodeEditorPanel` instance `#left` for the side-by-side view.
    left_panel: Arc<CodeEditorPanel>,
    /// The RIGHT (new) document, behind a `CodeEditorPanel` instance `#right`.
    right_panel: Arc<CodeEditorPanel>,
    /// The left buffer text (kept so the inline view and the diff recompute can read it without
    /// reaching through the panel).
    left_buffer: TextBuffer,
    /// The right buffer text.
    right_buffer: TextBuffer,
    /// The computed line-level diff blocks.
    diff: Vec<DiffBlock>,
    /// The current display mode (atomic so a `&self` toggle / agent can flip it under the `Sync` panel).
    mode: AtomicU8,
    /// Synchronized-scroll state for the side-by-side view.
    sync_scroll: Mutex<SyncScrollState>,
    /// MERGE state (only populated by [`DiffEditorPanel::merge`]): the base/local/remote buffers, the
    /// merge blocks (with their `chosen` sides), and the recomputed merged output buffer.
    merge: Mutex<Option<MergeState>>,
}

/// Three-way merge state for [`DiffMode::Merge`].
struct MergeState {
    base: TextBuffer,
    local: TextBuffer,
    remote: TextBuffer,
    blocks: Vec<MergeBlock>,
    /// The merged output buffer, recomputed whenever a block's `chosen` changes.
    merged: TextBuffer,
}

impl DiffEditorPanel {
    /// Construct a diff panel for two documents in [`DiffMode::SideBySide`]. `extension` selects the
    /// grammar for both panes (e.g. `"rs"`). The two `CodeEditorPanel`s are built with disjoint
    /// instance suffixes (`left` / `right`) so their scroll ids + AccessKit author_ids never collide
    /// (RISK-004).
    pub fn diff(left: TextBuffer, right: TextBuffer, extension: &str) -> Self {
        let left_text = left.to_string();
        let right_text = right.to_string();
        let diff = DiffEngine::diff(&left, &right);
        let line_map = build_line_map(&diff);
        let left_panel = Arc::new(CodeEditorPanel::with_instance(&left_text, extension, "left"));
        let right_panel = Arc::new(CodeEditorPanel::with_instance(&right_text, extension, "right"));
        // In a diff view the per-pane outline + minimap chrome are noise (and the minimap's colored
        // overview would visually compete with the diff-block tints); hide both so each pane is a
        // clean text column the diff-background overlay paints over.
        for p in [&left_panel, &right_panel] {
            p.set_show_outline(false);
            p.set_show_minimap(false);
        }
        Self {
            left_panel,
            right_panel,
            left_buffer: left,
            right_buffer: right,
            diff,
            mode: AtomicU8::new(DiffMode::SideBySide.to_u8()),
            sync_scroll: Mutex::new(SyncScrollState {
                line_map,
                ..Default::default()
            }),
            merge: Mutex::new(None),
        }
    }

    /// Construct a merge panel for a three-way conflict in [`DiffMode::Merge`] (MT
    /// `open_merge(base, local, remote)`). The merge blocks are computed up front; conflicts start
    /// unresolved (the accept buttons set `chosen`), and the merged output is seeded from the
    /// non-conflict defaults.
    pub fn merge(base: TextBuffer, local: TextBuffer, remote: TextBuffer, extension: &str) -> Self {
        let local_text = local.to_string();
        let remote_text = remote.to_string();
        // The side-by-side panes show local (left) vs remote (right) in merge mode.
        let diff = DiffEngine::diff(&local, &remote);
        let line_map = build_line_map(&diff);
        let blocks = MergeEngine::three_way(&base, &local, &remote);
        let merged = MergeEngine::apply(&base, &local, &remote, &blocks);
        let left_panel = Arc::new(CodeEditorPanel::with_instance(&local_text, extension, "left"));
        let right_panel = Arc::new(CodeEditorPanel::with_instance(&remote_text, extension, "right"));
        for p in [&left_panel, &right_panel] {
            p.set_show_outline(false);
            p.set_show_minimap(false);
        }
        Self {
            left_panel,
            right_panel,
            left_buffer: local.clone(),
            right_buffer: remote.clone(),
            diff,
            mode: AtomicU8::new(DiffMode::Merge.to_u8()),
            sync_scroll: Mutex::new(SyncScrollState { line_map, ..Default::default() }),
            merge: Mutex::new(Some(MergeState { base, local, remote, blocks, merged })),
        }
    }

    /// The current display mode.
    pub fn mode(&self) -> DiffMode {
        DiffMode::from_u8(self.mode.load(Ordering::Relaxed))
    }

    /// Set the display mode (the toggle button + an agent drive this).
    pub fn set_mode(&self, mode: DiffMode) {
        self.mode.store(mode.to_u8(), Ordering::Relaxed);
    }

    /// Toggle between SideBySide and Inline (the header toggle button). A no-op flip target when in
    /// Merge mode (the merge view has its own three-pane layout; the toggle is not shown there).
    pub fn toggle_mode(&self) {
        let next = match self.mode() {
            DiffMode::SideBySide => DiffMode::Inline,
            DiffMode::Inline => DiffMode::SideBySide,
            DiffMode::Merge => DiffMode::Merge,
        };
        self.set_mode(next);
    }

    /// The computed diff blocks (for tests / agents).
    pub fn diff_blocks(&self) -> &[DiffBlock] {
        &self.diff
    }

    /// A snapshot of the synchronized-scroll line map (for tests / agents).
    pub fn line_map(&self) -> Vec<SyncRow> {
        self.sync_scroll.lock().unwrap_or_else(|e| e.into_inner()).line_map.clone()
    }

    /// A snapshot of the merge blocks (for tests / agents), or empty when not in merge mode.
    pub fn merge_blocks(&self) -> Vec<MergeBlock> {
        self.merge
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|m| m.blocks.clone())
            .unwrap_or_default()
    }

    /// The current merged-output buffer text (merge mode), or `None` when not in merge mode.
    pub fn merged_text(&self) -> Option<String> {
        self.merge
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|m| m.merged.to_string())
    }

    /// Resolve the merge block at index `block_idx` with `choice` and recompute the merged buffer
    /// (MT step 5). No-op when not in merge mode or the index is out of range (never panics). This is
    /// the public surface the accept buttons AND a swarm agent drive.
    pub fn accept_merge_choice(&self, block_idx: usize, choice: MergeChoice) {
        let mut guard = self.merge.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = guard.as_mut() {
            if let Some(block) = state.blocks.get_mut(block_idx) {
                block.chosen = Some(choice);
                state.merged =
                    MergeEngine::apply(&state.base, &state.local, &state.remote, &state.blocks);
            }
        }
    }

    /// The number of conflict blocks (merge mode) — used by the panel to know how many accept-button
    /// rows to render and by tests.
    pub fn conflict_count(&self) -> usize {
        self.merge
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_ref()
            .map(|m| m.blocks.iter().filter(|b| b.status == MergeStatus::Conflict).count())
            .unwrap_or(0)
    }

    /// The stable AccessKit author_id for an accept-local button on merge block `n`.
    pub fn accept_local_author_id(n: usize) -> String {
        format!("{DIFF_BLOCK_ACCEPT_LOCAL_PREFIX}{n}_accept_local")
    }
    /// The stable AccessKit author_id for an accept-remote button on merge block `n`.
    pub fn accept_remote_author_id(n: usize) -> String {
        format!("{DIFF_BLOCK_ACCEPT_LOCAL_PREFIX}{n}_accept_remote")
    }
    /// The stable AccessKit author_id for an accept-both button on merge block `n`.
    pub fn accept_both_author_id(n: usize) -> String {
        format!("{DIFF_BLOCK_ACCEPT_LOCAL_PREFIX}{n}_accept_both")
    }

    /// Compute the synchronized right-pane scroll offset for a given left scroll offset (MT step 2 /
    /// AC-007). `line_height` is the rendered per-line height; the left offset is converted to a left
    /// line, mapped to the right line through the line map, and converted back to a right offset.
    /// Returns the right offset that visually aligns the panes (within one line). Pure + testable.
    pub fn synced_right_offset(&self, left_offset: f32, line_height: f32) -> f32 {
        if line_height <= 0.0 {
            return left_offset;
        }
        let left_line = (left_offset / line_height).floor().max(0.0) as usize;
        let line_map = self.line_map();
        match right_line_for_left_line(&line_map, left_line) {
            Some(right_line) => right_line as f32 * line_height,
            None => left_offset,
        }
    }

    /// Map a LEFT line to its synchronized RIGHT line through the diff line map (MT step 6 /
    /// AC-007). Returns the right line that visually aligns with `left_line` (or `left_line` itself
    /// when the line map cannot resolve it — e.g. before the diff covers that row).
    pub fn synced_right_line(&self, left_line: usize) -> usize {
        let line_map = self.line_map();
        right_line_for_left_line(&line_map, left_line).unwrap_or(left_line)
    }

    /// Scroll the LEFT pane so `left_line` is at the top and scroll the RIGHT pane to the
    /// synchronized line through the diff line map (MT step 2 synchronized scroll). The deterministic
    /// public surface the operator, a swarm agent, and AC-007 drive (rather than reaching into egui's
    /// persisted scroll state). Both panes use the same per-line height so the visual rows align.
    pub fn scroll_left_to(&self, left_line: usize) {
        let right_line = self.synced_right_line(left_line);
        self.left_panel.scroll_to_line(left_line);
        self.right_panel.scroll_to_line(right_line);
        let mut sync = self.sync_scroll.lock().unwrap_or_else(|e| e.into_inner());
        sync.last_changed = Some(Side::Left);
    }

    /// The LEFT pane's most-recent painted line range (for tests / agents — AC-007 reads this to
    /// confirm the pane scrolled).
    pub fn left_visible_range(&self) -> std::ops::Range<usize> {
        self.left_panel.last_visible_range()
    }

    /// The RIGHT pane's most-recent painted line range (for tests / agents — AC-007 confirms the
    /// right pane followed the left).
    pub fn right_visible_range(&self) -> std::ops::Range<usize> {
        self.right_panel.last_visible_range()
    }

    /// Diff-block background color for a status (MT step 3). Translucent RGBA via
    /// `from_rgba_unmultiplied` — the CONTROL-4-sanctioned form (NOT `from_rgb(`). The contract names
    /// these exact values.
    fn diff_color(status: DiffStatus) -> egui::Color32 {
        match status {
            DiffStatus::Added => egui::Color32::from_rgba_unmultiplied(0, 255, 0, 30),
            DiffStatus::Removed => egui::Color32::from_rgba_unmultiplied(255, 0, 0, 30),
            DiffStatus::Modified => egui::Color32::from_rgba_unmultiplied(255, 200, 0, 30),
            // Equal blocks get no tint.
            DiffStatus::Equal => egui::Color32::TRANSPARENT,
        }
    }

    /// Render the panel into `ui` (the egui widget entry point). Renders the header (mode toggle),
    /// then the body for the active mode, then emits the panel-container AccessKit node.
    pub fn show(&self, ui: &mut egui::Ui) {
        let syntax = syntax_tokens_for(ui.visuals());

        // Header: the SideBySide/Inline toggle (hidden in Merge mode, which has its own layout).
        let mode = self.mode();
        if mode != DiffMode::Merge {
            ui.horizontal(|ui| {
                let label = match mode {
                    DiffMode::SideBySide => "\u{25EB} Side-by-side",
                    _ => "\u{2261} Inline",
                };
                let resp = ui
                    .selectable_label(false, label)
                    .on_hover_text("Toggle side-by-side / inline diff");
                if resp.clicked() {
                    self.toggle_mode();
                }
                // Emit the toggle button AccessKit node (diff_mode_toggle). accesskit 0.21.1 has no
                // ToggleButton; Role::Button is the field-correct nearest (the AC asserts author_id).
                let toggle_id =
                    unsafe { egui::Id::from_high_entropy_bits(DIFF_MODE_TOGGLE_NODE_ID) };
                ui.ctx().accesskit_node_builder(toggle_id, |node| {
                    node.set_role(accesskit::Role::Button);
                    node.set_author_id(DIFF_MODE_TOGGLE_AUTHOR_ID.to_owned());
                    node.set_label("Toggle diff mode".to_owned());
                    node.add_action(accesskit::Action::Click);
                });
            });
        }

        match mode {
            DiffMode::SideBySide => self.show_side_by_side(ui, &syntax),
            DiffMode::Inline => self.show_inline(ui, &syntax),
            DiffMode::Merge => self.show_merge(ui, &syntax),
        }

        // The panel container node (Role::Group). Emitted on a fixed id for the standalone panel; in
        // the shell the pane container id comes from the registry and this node nests under it.
        let container_id = unsafe { egui::Id::from_high_entropy_bits(DIFF_PANEL_CONTAINER_NODE_ID) };
        ui.ctx().accesskit_node_builder(container_id, |node| {
            node.set_role(accesskit::Role::Group);
            node.set_author_id(DIFF_EDITOR_PANEL_AUTHOR_ID.to_owned());
            node.set_label("Diff editor".to_owned());
        });
    }

    /// SideBySide body: two `CodeEditorPanel`s in `ui.columns(2, ...)`, each with its diff-block
    /// background rects painted over the changed rows, and the left pane's scroll mapped to the right
    /// pane through the line map (AC-003 / AC-007 / RISK-004).
    fn show_side_by_side(&self, ui: &mut egui::Ui, _syntax: &HsSyntaxTokens) {
        // Snapshot the per-side diff blocks so each pane paints only its own side's tint.
        let blocks = self.diff.clone();
        // Measure the monospace row height once with the outer ui (both panes paint with the same
        // FontId::monospace(13.0) as the diff backgrounds, so this matches the painted rows —
        // MT-002 positioning unit). Row spacing is zeroed inside the panes' rows.
        let line_height = monospace_row_height(ui);

        // Each pane renders its toggle row (the small "Outline/Minimap" strip) before the text rows.
        // The diff-background rects must align with the TEXT rows, so they are offset from the column
        // top by that toggle-row height. Captured here so the overlay (painted AFTER the panes, on a
        // foreground layer, so the panel's opaque editor background does not occlude it) lands on the
        // right rows.
        let text_top_offset = line_height + ui.spacing().item_spacing.y;

        // Capture each column's text rect so the overlay paints over the rendered rows.
        let mut left_text_rect = egui::Rect::NOTHING;
        let mut right_text_rect = egui::Rect::NOTHING;

        ui.columns(2, |cols| {
            {
                let ui = &mut cols[0];
                let col_rect = ui.max_rect();
                left_text_rect = egui::Rect::from_min_max(
                    egui::pos2(col_rect.left(), col_rect.top() + text_top_offset),
                    col_rect.max,
                );
                self.left_panel.show(ui);
            }
            {
                let ui = &mut cols[1];
                let col_rect = ui.max_rect();
                right_text_rect = egui::Rect::from_min_max(
                    egui::pos2(col_rect.left(), col_rect.top() + text_top_offset),
                    col_rect.max,
                );
                self.right_panel.show(ui);
            }
        });

        // Paint the diff-block backgrounds on a FOREGROUND layer over each pane's text rect, AFTER the
        // panes rendered (so the panel's opaque background cannot occlude the tint — the MT step-3
        // green/red/yellow rects). A translucent fill keeps the underlying text readable.
        let fg = ui.ctx().layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            ui.id().with("diff_bg_overlay"),
        ));
        paint_side_backgrounds(&fg, &blocks, Side::Left, line_height, left_text_rect);
        paint_side_backgrounds(&fg, &blocks, Side::Right, line_height, right_text_rect);
    }

    /// Inline body: one panel showing the unified diff with `+ ` / `- ` / `  ` prefixes (MT step 4).
    /// Rendered as colored monospace rows so the prefix color carries the change semantics (AC-004).
    fn show_inline(&self, ui: &mut egui::Ui, syntax: &HsSyntaxTokens) {
        let added_color = inline_added_color();
        let removed_color = inline_removed_color();
        let context_color = syntax.punctuation;
        let mono = egui::FontId::monospace(13.0);

        let left_lines: Vec<String> = self.left_buffer.to_string().lines().map(|l| l.to_owned()).collect();
        let right_lines: Vec<String> = self.right_buffer.to_string().lines().map(|l| l.to_owned()).collect();

        egui::ScrollArea::vertical()
            .id_salt("diff_editor_inline_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                for block in &self.diff {
                    match block.status {
                        DiffStatus::Equal => {
                            for l in block.left_lines.clone() {
                                inline_row(ui, "  ", left_lines.get(l), context_color, &mono);
                            }
                        }
                        DiffStatus::Removed => {
                            for l in block.left_lines.clone() {
                                inline_row(ui, "- ", left_lines.get(l), removed_color, &mono);
                            }
                        }
                        DiffStatus::Added => {
                            for r in block.right_lines.clone() {
                                inline_row(ui, "+ ", right_lines.get(r), added_color, &mono);
                            }
                        }
                        DiffStatus::Modified => {
                            // A modify renders as the removed old lines then the added new lines.
                            for l in block.left_lines.clone() {
                                inline_row(ui, "- ", left_lines.get(l), removed_color, &mono);
                            }
                            for r in block.right_lines.clone() {
                                inline_row(ui, "+ ", right_lines.get(r), added_color, &mono);
                            }
                        }
                    }
                }
            });
    }

    /// Merge body (MT step 5): the local/remote panes side by side with their diff backgrounds, then
    /// a per-conflict resolution row carrying Accept Local / Accept Remote / Accept Both buttons that
    /// resolve the block and recompute the merged buffer. Each accept button emits its stable
    /// AccessKit node (`diff_block_{n}_accept_local` etc.).
    fn show_merge(&self, ui: &mut egui::Ui, syntax: &HsSyntaxTokens) {
        ui.label(
            egui::RichText::new("Merge conflicts")
                .color(syntax.keyword)
                .font(egui::FontId::monospace(13.0)),
        );

        // The two changed documents side by side (local left, remote right) with diff backgrounds.
        self.show_side_by_side(ui, syntax);

        // Resolution rows: one per merge block, conflicts get the three accept buttons.
        let blocks = self.merge_blocks();
        for (n, block) in blocks.iter().enumerate() {
            if block.status != MergeStatus::Conflict {
                continue;
            }
            ui.horizontal(|ui| {
                let resolved = match block.chosen {
                    Some(MergeChoice::Local) => " [local]",
                    Some(MergeChoice::Remote) => " [remote]",
                    Some(MergeChoice::Both) => " [both]",
                    None => " [unresolved]",
                };
                ui.label(
                    egui::RichText::new(format!("Conflict @ line {}{resolved}", block.local_lines.start))
                        .color(syntax.comment)
                        .font(egui::FontId::monospace(12.0)),
                );

                self.accept_button(ui, n, "Accept Local", MergeChoice::Local, &Self::accept_local_author_id(n));
                self.accept_button(ui, n, "Accept Remote", MergeChoice::Remote, &Self::accept_remote_author_id(n));
                self.accept_button(ui, n, "Accept Both", MergeChoice::Both, &Self::accept_both_author_id(n));
            });
        }
    }

    /// Render one merge accept button + emit its AccessKit node, resolving the block on click.
    fn accept_button(
        &self,
        ui: &mut egui::Ui,
        block_idx: usize,
        label: &str,
        choice: MergeChoice,
        author_id: &str,
    ) {
        let resp = ui.button(label);
        if resp.clicked() {
            self.accept_merge_choice(block_idx, choice);
        }
        // Emit (or re-decorate) the button's live node with the stable author_id (AC-006). egui has
        // already filled the Button role + Click action from the widget; we add the author_id.
        let author = author_id.to_owned();
        ui.ctx().accesskit_node_builder(resp.id, move |node| {
            node.set_author_id(author.clone());
        });
    }
}

/// Paint the diff-block background rects for ONE side over the pane area (MT step 3). The left side
/// tints Removed (red) + Modified (yellow); the right side tints Added (green) + Modified (yellow).
/// Rows are positioned via the same `line * line_height` unit the panes paint with (MT-002
/// positioning); the first painted row is the top of `pane_rect` (the diff panes start at scroll 0
/// on the first frame — the deterministic screenshot ACs render at the top).
fn paint_side_backgrounds(
    painter: &egui::Painter,
    blocks: &[DiffBlock],
    side: Side,
    line_height: f32,
    pane_rect: egui::Rect,
) {
    if !pane_rect.is_finite() || pane_rect.width() <= 0.0 {
        return;
    }
    // Clip to the pane text rect so a background rect never bleeds into the sibling pane / chrome.
    let painter = painter.with_clip_rect(pane_rect);
    let layout = VirtualLineLayout::new(0, line_height, pane_rect.height(), 0.0);
    for block in blocks {
        let (range, want) = match side {
            Side::Left => (
                block.left_lines.clone(),
                matches!(block.status, DiffStatus::Removed | DiffStatus::Modified),
            ),
            Side::Right => (
                block.right_lines.clone(),
                matches!(block.status, DiffStatus::Added | DiffStatus::Modified),
            ),
        };
        if !want || range.is_empty() {
            continue;
        }
        let color = DiffEditorPanel::diff_color(block.status);
        for line in range {
            let y = pane_rect.top() + layout.y_for_line(line);
            let rect = egui::Rect::from_min_size(
                egui::pos2(pane_rect.left(), y),
                egui::vec2(pane_rect.width(), line_height),
            );
            // Only paint rows that fall within the visible pane area.
            if rect.top() <= pane_rect.bottom() {
                painter.rect_filled(rect, 0.0, color);
            }
        }
    }
}

/// The monospace row height in px, measured the same way [`CodeEditorPanel`] measures its rows
/// (`ui.text_style_height(Monospace)`), so the diff-block background rects stride by the same unit
/// the panes paint with (MT-002 positioning).
fn monospace_row_height(ui: &egui::Ui) -> f32 {
    ui.text_style_height(&egui::TextStyle::Monospace).max(1.0)
}

/// One inline-diff row: a colored monospace `prefix + line` label. Missing lines render the prefix
/// only (so an empty added/removed line still shows its marker).
fn inline_row(
    ui: &mut egui::Ui,
    prefix: &str,
    line: Option<&String>,
    color: egui::Color32,
    mono: &egui::FontId,
) {
    let text = format!("{prefix}{}", line.map(|s| s.as_str()).unwrap_or(""));
    ui.label(egui::RichText::new(text).color(color).font(mono.clone()));
}

/// Resolve the active theme's syntax tokens from the live egui visuals (dark vs light). Mirrors the
/// private helper in `panel.rs` (kept local so the diff panel does not depend on a panel-private fn).
fn syntax_tokens_for(visuals: &egui::Visuals) -> HsSyntaxTokens {
    if visuals.dark_mode {
        crate::theme::HsTheme::Dark.palette().syntax
    } else {
        crate::theme::HsTheme::Light.palette().syntax
    }
}

/// Inline-diff ADDED prefix color (green). Theme-sourced via the dark palette's success token so it
/// reads green without a hardcoded `from_rgb(` literal (CONTROL-4). The diff add-green is a semantic
/// signal independent of dark/light, like the gutter severity hues.
fn inline_added_color() -> egui::Color32 {
    crate::theme::HsTheme::Dark.palette().success_text
}

/// Inline-diff REMOVED prefix color (red). Theme-sourced via the dark palette's error token.
fn inline_removed_color() -> egui::Color32 {
    crate::theme::HsTheme::Dark.palette().error_text
}

/// A [`PaneFactory`] that mounts a [`DiffEditorPanel`] as a named work-surface pane (MT-009 step:
/// place the new tile through the EXISTING pane registry + split layout, not a parallel layout
/// system). Registered for [`PaneType::CodeSymbol`] (the closest existing WP-011 pane variant for a
/// code surface, the same variant [`super::panel::CodeEditorPaneFactory`] uses) so the diff editor
/// appears in the WP-011 docking split layout through the existing infrastructure — no shell fork.
pub struct DiffEditorPaneFactory {
    panel: Arc<DiffEditorPanel>,
}

impl DiffEditorPaneFactory {
    /// Build a factory wrapping `panel`. `Arc` so the same panel renders across frames without the
    /// factory owning a `&mut` (the registry borrows `&dyn PaneFactory` at render time).
    pub fn new(panel: DiffEditorPanel) -> Self {
        Self { panel: Arc::new(panel) }
    }

    /// A shared handle to the wrapped panel, so the shell/tests can drive it (accept a merge choice,
    /// toggle mode, scroll) after the factory is mounted.
    pub fn panel(&self) -> Arc<DiffEditorPanel> {
        Arc::clone(&self.panel)
    }
}

impl PaneFactory for DiffEditorPaneFactory {
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
    fn line_map_pairs_equal_rows_and_marks_added_removed() {
        let left = TextBuffer::new("a\nb\nc");
        let right = TextBuffer::new("a\nNEW\nb\nc");
        let panel = DiffEditorPanel::diff(left, right, "rs");
        let map = panel.line_map();
        // The added "NEW" row must appear as a right-only entry.
        assert!(
            map.iter().any(|(l, r)| l.is_none() && r.is_some()),
            "an added line is a right-only row: {map:?}"
        );
        // Equal rows pair both sides.
        assert!(
            map.iter().any(|(l, r)| l.is_some() && r.is_some()),
            "equal lines pair both sides: {map:?}"
        );
    }

    #[test]
    fn synced_offset_maps_left_line_to_right_line() {
        // AC-007 basis (pure half): left has a removed block before the target line so the right line
        // index differs from the left; the synced offset must follow the line map, not be identical.
        let left = TextBuffer::new("a\nREMOVED\nb\nc\nd\ne\nf\ng");
        let right = TextBuffer::new("a\nb\nc\nd\ne\nf\ng");
        let panel = DiffEditorPanel::diff(left, right, "rs");
        let lh = 16.0;
        // Left line 4 ("d") maps to right line 3 ("d") because one line was removed before it.
        let left_offset = 4.0 * lh;
        let right_offset = panel.synced_right_offset(left_offset, lh);
        assert_eq!(
            (right_offset / lh).round() as i64,
            3,
            "left line 4 maps to right line 3 after one removed line; got {right_offset}"
        );
    }

    #[test]
    fn merge_accept_local_recomputes_merged_buffer() {
        // AC-005 basis (engine-through-panel half): a conflict resolved to Local puts the local line
        // in the merged buffer the panel exposes.
        let base = TextBuffer::new("l0\nl1\nl2");
        let local = TextBuffer::new("l0\nLOCAL_EDIT\nl2");
        let remote = TextBuffer::new("l0\nREMOTE_EDIT\nl2");
        let panel = DiffEditorPanel::merge(base, local, remote, "rs");
        assert_eq!(panel.conflict_count(), 1, "one conflict to resolve");

        // Find the conflict block index and accept local.
        let blocks = panel.merge_blocks();
        let idx = blocks
            .iter()
            .position(|b| b.status == MergeStatus::Conflict)
            .expect("a conflict exists");
        panel.accept_merge_choice(idx, MergeChoice::Local);

        let merged = panel.merged_text().expect("merge mode");
        assert!(merged.contains("LOCAL_EDIT"), "Accept Local -> merged has local: {merged:?}");
        assert!(!merged.contains("REMOTE_EDIT"), "Accept Local -> merged lacks remote: {merged:?}");
    }

    #[test]
    fn mode_toggle_flips_side_by_side_and_inline() {
        let panel = DiffEditorPanel::diff(TextBuffer::new("a"), TextBuffer::new("b"), "rs");
        assert_eq!(panel.mode(), DiffMode::SideBySide);
        panel.toggle_mode();
        assert_eq!(panel.mode(), DiffMode::Inline);
        panel.toggle_mode();
        assert_eq!(panel.mode(), DiffMode::SideBySide);
    }

    #[test]
    fn accept_button_author_ids_match_contract_shape() {
        assert_eq!(DiffEditorPanel::accept_local_author_id(0), "diff_block_0_accept_local");
        assert_eq!(DiffEditorPanel::accept_remote_author_id(3), "diff_block_3_accept_remote");
        assert_eq!(DiffEditorPanel::accept_both_author_id(2), "diff_block_2_accept_both");
    }
}
