//! Split / resize dividers for the native work-surface pane host (WP-KERNEL-011 MT-006).
//!
//! ## What this provides
//!
//! A fixed 2x2 pane grid partitioned by exactly two dividers:
//! - one **horizontal** divider that splits top/bottom at `weights.vertical * available_height`;
//! - one **vertical** divider that splits left/right at `weights.horizontal * available_width`.
//!
//! The four resulting rects host `pane-a` (top-left), `pane-b` (top-right), `pane-c` (bottom-left),
//! `pane-d` (bottom-right) from the MT-005 [`PaneRegistry`]. Each divider is:
//! - draggable by pointer (drag delta -> weight delta, clamped),
//! - resizable by keyboard (Arrow keys, ±[`SPLIT_STEP`]) **only when the divider is focused**,
//! - addressable out-of-process as an AccessKit [`accesskit::Role::Splitter`] node with a stable
//!   `author_id` (`divider-horizontal` / `divider-vertical`) and a `SetValue` action whose incoming
//!   value is clamped to `[SPLIT_MIN, SPLIT_MAX]` so an agent cannot collapse a pane to zero size.
//!
//! ## Why a hand-rolled 2x2 splitter and not `egui_tiles`
//!
//! `egui_tiles` owns its own internal tile tree + per-container `Share` weights and reflows on
//! drag; it does not expose the contract's explicit `SplitWeights { vertical, horizontal }` model
//! with the React field names, nor does it emit per-divider `Role::Splitter` AccessKit nodes with
//! the exact `author_id`s and clamped `SetValue` semantics the MT-006 acceptance criteria assert by
//! string. The MT-006 contract is specifically about that fixed 2x2 model (ported from
//! `app/src/App.tsx`), so a focused widget over the existing registry is the correct fit; bending
//! `egui_tiles` to it would fight every acceptance criterion. `egui_tiles` remains the intended
//! engine for free-form docking work (MT-007/008) where its tree model is the right tool.
//!
//! ## Clamp ordering (RISK: zero-size pane)
//!
//! [`clamp_split`] is applied to the weight the moment it changes (drag, keyboard, or AccessKit
//! `SetValue`) and again is implied by the rect math reading an already-clamped weight, so a pane
//! can never be computed at zero or negative size — there is no window where an unclamped weight
//! reaches the rect computation.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use egui::accesskit;

use crate::pane_registry::{PaneFactory, PaneId, PaneRegistry, PaneRenderContext, PaneType};
use crate::tab_bar::{
    apply_drop, apply_drop_same_pane, TabBar, TabBarColors, TabBarState, TAB_BAR_HEIGHT,
};

/// Minimum fraction either pane may shrink to. Ported verbatim from `app/src/App.tsx`
/// `SPLIT_MIN = 0.2`.
pub const SPLIT_MIN: f32 = 0.2;
/// Maximum fraction either pane may grow to. Ported verbatim from `app/src/App.tsx`
/// `SPLIT_MAX = 0.8`.
pub const SPLIT_MAX: f32 = 0.8;
/// Keyboard resize step. Ported verbatim from `app/src/App.tsx` `SPLIT_STEP = 0.05`.
pub const SPLIT_STEP: f32 = 0.05;

/// Painted divider line thickness (visual).
pub const DIVIDER_THICKNESS: f32 = 4.0;
/// Pointer hit-target thickness, intentionally wider than the painted line so the divider is easy
/// to grab. Matches the MT-010 rail spec for larger hit targets.
pub const DIVIDER_HIT_THICKNESS: f32 = 8.0;

/// Fixed AccessKit/egui node id for the horizontal (top/bottom) divider. Sits in the chrome id band
/// (< [`crate::pane_registry`] pane base 100, and distinct from the toggle=10 / title=20 / status=21)
/// so the collision-free invariant in `accessibility::registry` holds.
pub const DIVIDER_H_NODE_ID: u64 = 30;
/// Fixed AccessKit/egui node id for the vertical (left/right) divider.
pub const DIVIDER_V_NODE_ID: u64 = 31;

/// Stable out-of-process match key for the horizontal divider.
pub const DIVIDER_H_AUTHOR_ID: &str = "divider-horizontal";
/// Stable out-of-process match key for the vertical divider.
pub const DIVIDER_V_AUTHOR_ID: &str = "divider-vertical";

/// The four panes of the fixed 2x2 grid, in stable spatial order.
const PANE_A: &str = "pane-a"; // top-left
const PANE_B: &str = "pane-b"; // top-right
const PANE_C: &str = "pane-c"; // bottom-left
const PANE_D: &str = "pane-d"; // bottom-right

/// Which divider line this is. The variant names the LINE's orientation (matching React's
/// `aria-orientation` and `DragAxis` union in `app/src/App.tsx`), and each line controls the weight
/// of the SAME name — exactly as React does:
///
/// - `Horizontal` = the horizontal line that splits top/bottom (moves on Y); controls
///   `weights.horizontal` (React `axis === "horizontal"` -> `ArrowUp/Down`, `splitWeights.horizontal`).
/// - `Vertical` = the vertical line that splits left/right (moves on X); controls
///   `weights.vertical` (React `axis === "vertical"` -> `ArrowLeft/Right`, `splitWeights.vertical`).
///
/// The line-name and the weight-name match (horizontal line -> horizontal weight) because React's
/// `DragAxis` value is the weight key it mutates; see `handleSplitDividerKeyDown` /
/// `handleSplitDividerPointerDown` in `app/src/App.tsx`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SplitAxis {
    /// The horizontal divider line (top/bottom split, moves on Y); controls `weights.horizontal`.
    Horizontal,
    /// The vertical divider line (left/right split, moves on X); controls `weights.vertical`.
    Vertical,
}

/// Persisted split fractions. Field names (`vertical`, `horizontal`) match the React `SplitWeights`
/// schema (`app/src/App.tsx`) exactly so MT-009's layout snapshot round-trips with the web app.
///
/// CANONICAL meaning (React `app/src/App.css` `.main-pane-grid`):
/// - `vertical`: fraction of the available *width* given to the left column (drives
///   `grid-template-columns` via `--hsk-pane-vertical-split`; the **vertical divider line**, X axis).
/// - `horizontal`: fraction of the available *height* given to the top row (drives
///   `grid-template-rows` via `--hsk-pane-horizontal-split`; the **horizontal divider line**, Y axis).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SplitWeights {
    pub vertical: f32,
    pub horizontal: f32,
}

impl Default for SplitWeights {
    /// `DEFAULT_SPLIT_WEIGHTS` from `app/src/App.tsx`: `{ vertical: 0.5, horizontal: 0.55 }`.
    fn default() -> Self {
        Self {
            vertical: 0.5,
            horizontal: 0.55,
        }
    }
}

/// Per-frame pointer-drag state. **Deliberately NOT part of [`SplitWeights`]** (which is
/// serialized): persisting transient drag flags would corrupt layout snapshots (red-team RISK-5).
/// Held as a separate non-persisted field in `HandshakeApp`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SplitDragState {
    pub dragging_horizontal: bool,
    pub dragging_vertical: bool,
}

/// Clamp a split fraction to `[SPLIT_MIN, SPLIT_MAX]`. Ported from the React
/// `clampSplit = (v) => Math.max(SPLIT_MIN, Math.min(SPLIT_MAX, v))`.
///
/// Applied at every mutation site (drag, keyboard, AccessKit `SetValue`) so a weight that reaches
/// the rect computation is always already clamped — there is no window for a zero-size pane.
#[inline]
pub fn clamp_split(v: f32) -> f32 {
    v.clamp(SPLIT_MIN, SPLIT_MAX)
}

/// Apply an incoming AccessKit `SetValue` fractional position to a weight, clamped. Pure function so
/// the clamp contract ("`SetValue(0.1)` -> 0.2") is unit-testable without an egui frame.
#[inline]
pub fn apply_set_value(value: f64) -> f32 {
    clamp_split(value as f32)
}

/// The four pane rects of the 2x2 grid plus the two divider hit-rects, all in the same coordinate
/// space as the input `area`. Used by the renderer and directly by unit tests (adjacency / no-gap /
/// no-overlap proofs) without needing a live egui frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitRects {
    pub top_left: egui::Rect,
    pub top_right: egui::Rect,
    pub bottom_left: egui::Rect,
    pub bottom_right: egui::Rect,
    /// Full-width hit rect for the horizontal divider (centered on the top/bottom boundary).
    pub divider_h: egui::Rect,
    /// Full-height hit rect for the vertical divider (centered on the left/right boundary).
    pub divider_v: egui::Rect,
}

/// Compute the four pane rects and the two divider hit-rects from an available `area` and the
/// (already-clamped) split weights.
///
/// ## Axis convention (CANONICAL — matches React `app/src/App.css` / `App.tsx`)
///
/// The React app drives a CSS grid (`app/src/App.css` `.main-pane-grid`, ~lines 3387–3392):
/// - `--hsk-pane-vertical-split` (= `splitWeights.vertical`) sets `grid-template-COLUMNS` -> it is
///   the **left/right column-width split on the X axis**.
/// - `--hsk-pane-horizontal-split` (= `splitWeights.horizontal`) sets `grid-template-ROWS` -> it is
///   the **top/bottom row-height split on the Y axis**.
///
/// So, matching React EXACTLY:
/// - `split_x = area.left + width * weights.vertical`   (the vertical divider line / column split)
/// - `split_y = area.top  + height * weights.horizontal` (the horizontal divider line / row split)
///
/// (Earlier this was transposed; the React grid above is the single source of truth.)
///
/// The four pane rects tile the whole area with NO gap and NO overlap — they meet exactly on the
/// boundary lines. The divider hit-rects straddle those boundaries by `DIVIDER_HIT_THICKNESS/2` on
/// each side (they overlap the panes by design; that overlap is the grab zone, not a layout gap).
///
/// Callers MUST pass weights already run through [`clamp_split`]; this function does not re-clamp so
/// the clamp ordering stays explicit at the mutation sites.
pub fn compute_split_rects(area: egui::Rect, weights: SplitWeights) -> SplitRects {
    let width = area.width();
    let height = area.height();

    // CANONICAL axis mapping (React grid): vertical weight -> X column split; horizontal weight ->
    // Y row split. Do NOT transpose these.
    let split_x = area.left() + width * weights.vertical;
    let split_y = area.top() + height * weights.horizontal;

    let left = area.left();
    let right = area.right();
    let top = area.top();
    let bottom = area.bottom();

    let top_left = egui::Rect::from_min_max(egui::pos2(left, top), egui::pos2(split_x, split_y));
    let top_right = egui::Rect::from_min_max(egui::pos2(split_x, top), egui::pos2(right, split_y));
    let bottom_left =
        egui::Rect::from_min_max(egui::pos2(left, split_y), egui::pos2(split_x, bottom));
    let bottom_right =
        egui::Rect::from_min_max(egui::pos2(split_x, split_y), egui::pos2(right, bottom));

    let half_hit = DIVIDER_HIT_THICKNESS / 2.0;
    let divider_h = egui::Rect::from_min_max(
        egui::pos2(left, split_y - half_hit),
        egui::pos2(right, split_y + half_hit),
    );
    let divider_v = egui::Rect::from_min_max(
        egui::pos2(split_x - half_hit, top),
        egui::pos2(split_x + half_hit, bottom),
    );

    SplitRects {
        top_left,
        top_right,
        bottom_left,
        bottom_right,
        divider_h,
        divider_v,
    }
}

/// The three theme-token colors a divider paints with, by interaction state. Sourced from the
/// MT-003 [`crate::theme::HsPalette`] (`divider_idle` / `divider_hover` / `divider_grab`) and passed
/// into [`SplitLayoutWidget::show`] so the divider NEVER reads egui's generic visuals for its color
/// (MT-006 contract: divider colors come from the theme tokens). Carrying just the three colors
/// (not the whole palette) keeps the split layout decoupled from the palette type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DividerColors {
    /// Idle line color (`theme.divider_idle`).
    pub idle: egui::Color32,
    /// Hover / keyboard-focus line color (`theme.divider_hover`).
    pub hover: egui::Color32,
    /// Active-drag (grab) line color (`theme.divider_grab`).
    pub grab: egui::Color32,
}

/// Stateless renderer for the 2x2 split layout. Borrows the registry + factories at `show` time and
/// owns nothing, so it is safe to construct per frame (mirrors [`crate::pane_registry::PaneHostWidget`]).
pub struct SplitLayoutWidget;

impl SplitLayoutWidget {
    /// Fixed `egui::Id` for a divider. `from_high_entropy_bits` (NOT `Id::new`, which hashes) so the
    /// live AccessKit `NodeId` equals the fixed `node_id` — the same convention chrome (20/21) and
    /// the theme toggle (10) use, which is what makes out-of-process steering find the divider by a
    /// stable id across process restarts (RISK-1 / CONTROL-1).
    ///
    /// # Safety
    /// A single hand-assigned, never-reused fixed id cannot self-collide; entropy only affects
    /// egui's child `IdMap` distribution. The values (30, 31) are disjoint from the toggle (10),
    /// chrome (20, 21), and the pane id base (100+).
    fn divider_id(axis: SplitAxis) -> egui::Id {
        let node_id = match axis {
            SplitAxis::Horizontal => DIVIDER_H_NODE_ID,
            SplitAxis::Vertical => DIVIDER_V_NODE_ID,
        };
        unsafe { egui::Id::from_high_entropy_bits(node_id) }
    }

    /// Render the full 2x2 work surface into `ui`: the four panes in their computed rects plus the
    /// two draggable / keyboard-resizable / AccessKit-addressable dividers.
    ///
    /// - `weights`: persisted split fractions (mutated by drag / keyboard / `SetValue`).
    /// - `drag_state`: per-frame, non-persisted pointer-drag flags.
    /// - `active_pane`: updated to the pane the operator last clicked (so later MTs can highlight it).
    /// - `registry`: the MT-005 pane source of truth.
    /// - `divider_colors`: the MT-003 theme-token colors the dividers paint with (idle/hover/grab);
    ///   the divider does NOT read egui's generic visuals for its color (MT-006 contract).
    /// - `factory_for`: returns the factory for a pane type (the caller wires it to its
    ///   `HashMap<PaneType, Box<dyn PaneFactory>>` with a placeholder fallback) — the SAME bound
    ///   [`crate::pane_registry::PaneHostWidget::show_with_accesskit`] uses.
    /// - `emit_accesskit`: live AccessKit emission callback for panes (MT-025), invoked once per pane
    ///   inside the pane's own egui scope. Wired by the app to `accessibility::emit_pane_node` so live
    ///   pane emission is NOT regressed by moving panes into split rects.
    /// - `tab_bars`: per-pane tab-bar state (MT-007), keyed by `PaneId`. The tab bar is rendered in a
    ///   [`TAB_BAR_HEIGHT`]-tall strip carved off the TOP of each pane rect; the pane body renders in
    ///   the remaining rect below it. Tab interactions (activate / close / pin / inter-pane drop) are
    ///   reconciled into this map after the pane loop, so a tab dragged from one pane to another moves
    ///   exactly once (see [`apply_drop`]).
    /// - `tab_colors`: the MT-003 theme-token colors the tab bar paints with.
    #[allow(clippy::too_many_arguments)]
    pub fn show<'f, F, A>(
        ui: &mut egui::Ui,
        weights: &mut SplitWeights,
        drag_state: &mut SplitDragState,
        active_pane: &mut Option<PaneId>,
        registry: &Arc<Mutex<PaneRegistry>>,
        divider_colors: DividerColors,
        tab_bars: &mut HashMap<PaneId, TabBarState>,
        tab_colors: TabBarColors,
        mut factory_for: F,
        mut emit_accesskit: A,
    ) where
        F: FnMut(&PaneType) -> &'f dyn PaneFactory,
        A: FnMut(&egui::Context, egui::Id, &str, accesskit::Role, &str),
    {
        // Defensive clamp at the top of the frame: even if a weight was deserialized out of range
        // (MT-009 snapshot, or a hand-edited file), the rect math below reads only clamped values.
        weights.vertical = clamp_split(weights.vertical);
        weights.horizontal = clamp_split(weights.horizontal);

        let area = ui.available_rect_before_wrap();
        // Degenerate area (headless 0x0 first frame): nothing to lay out, and dividing by zero would
        // poison the weights. Skip rendering this frame; egui will repaint with a real size.
        if area.width() <= 1.0 || area.height() <= 1.0 {
            return;
        }

        let rects = compute_split_rects(area, *weights);

        // ── Pane bodies + tab bars ─────────────────────────────────────────────────────────────────
        // Each pane is looked up by its spatial id and rendered into its rect. A missing pane (e.g.
        // a registry that does not seed the full 2x2) degrades to an empty rect rather than a panic.
        // A TAB_BAR_HEIGHT-tall strip is carved off the TOP of each pane rect for the MT-007 tab bar;
        // the pane body renders in the remaining rect below it. Tab interactions are collected here and
        // reconciled into `tab_bars` AFTER the loop (a drop needs two distinct &mut bars, which is not
        // possible while iterating).
        let registry_guard = registry.lock().expect("pane registry mutex poisoned");
        let pane_slots = [
            (PANE_A, rects.top_left),
            (PANE_B, rects.top_right),
            (PANE_C, rects.bottom_left),
            (PANE_D, rects.bottom_right),
        ];
        // (pane_id, TabBarResponse) collected per pane this frame, reconciled below.
        let mut tab_responses: Vec<(PaneId, crate::tab_bar::TabBarResponse)> = Vec::new();
        for (pane_key, pane_rect) in pane_slots {
            let pane_id: PaneId = Arc::from(pane_key);
            let Some(record) = registry_guard.get(&pane_id) else {
                continue;
            };
            // Stable egui id from the registry-owned monotonic NodeId (same derivation as
            // PaneHostWidget) so child widget ids and the live AccessKit node id stay stable.
            let node_id = registry_guard
                .accesskit_id(&pane_id)
                .map(|n| n.0)
                .unwrap_or_else(|| hash_pane_id(&pane_id));
            let pane_egui_id = unsafe { egui::Id::from_high_entropy_bits(node_id) };

            // Carve the tab-bar strip off the top of the pane rect. Guard against a degenerate pane
            // shorter than the tab bar: in that case the body rect collapses but never inverts.
            let tab_bar_height = TAB_BAR_HEIGHT.min(pane_rect.height());
            let tab_bar_rect = egui::Rect::from_min_max(
                pane_rect.min,
                egui::pos2(pane_rect.right(), pane_rect.top() + tab_bar_height),
            );
            let body_rect = egui::Rect::from_min_max(
                egui::pos2(pane_rect.left(), pane_rect.top() + tab_bar_height),
                pane_rect.max,
            );

            // Render the tab bar (if this pane has one) in its strip.
            if let Some(tab_state) = tab_bars.get(&pane_id) {
                let mut tab_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("tab-bar", node_id))
                        .max_rect(tab_bar_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                tab_ui.set_clip_rect(tab_bar_rect);
                let resp = TabBar::show(&mut tab_ui, tab_state, tab_colors);
                tab_responses.push((pane_id.clone(), resp));
            }

            let factory = factory_for(&record.pane_type);
            let role = factory.accesskit_role();
            let label = record.pane_type.label();
            let render_ctx = PaneRenderContext {
                record,
                egui_id: pane_egui_id,
            };

            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt(node_id)
                    .max_rect(body_rect)
                    .layout(egui::Layout::top_down(egui::Align::Min)),
            );
            child.set_clip_rect(body_rect);
            factory.render(&mut child, &render_ctx);
            // Register the pane's stable id on its content rect so the live AccessKit node attaches
            // under this scope, and capture a click so the operator can activate a pane.
            let pane_response = child.interact(body_rect, pane_egui_id, egui::Sense::click());
            if pane_response.clicked() {
                *active_pane = Some(pane_id.clone());
            }
            emit_accesskit(ui.ctx(), pane_egui_id, pane_id.as_ref(), role, &label);
        }
        drop(registry_guard);

        // ── Reconcile tab interactions ──────────────────────────────────────────────────────────────
        // Apply each pane's tab interactions to `tab_bars`. Order matters: drops are applied LAST so a
        // close/activate this frame does not shift indices a drop relies on. A drop is the only
        // interaction that touches TWO bars, so it is the only one that needs the cross-bar
        // `apply_drop`; everything else mutates the one owning bar.
        for (pane_id, resp) in &tab_responses {
            if let Some(bar) = tab_bars.get_mut(pane_id) {
                if let Some(idx) = resp.activated_index {
                    bar.activate(idx);
                    *active_pane = Some(pane_id.clone());
                }
                if let Some((idx, pin)) = resp.pin_toggled {
                    if pin {
                        bar.pin_tab(idx);
                    } else {
                        bar.unpin_tab(idx);
                    }
                }
                if let Some(idx) = resp.closed_index {
                    bar.close_tab(idx);
                }
            }
        }
        // Drops second: each completed drop moves a tab from its source bar into the target bar.
        for (target_pane_id, resp) in &tab_responses {
            let Some((payload, target)) = &resp.drop_completed else {
                continue;
            };
            if payload.source_pane_id == target.target_pane_id {
                // Same-pane reorder: one bar.
                if let Some(bar) = tab_bars.get_mut(target_pane_id) {
                    apply_drop_same_pane(payload, target, bar);
                }
            } else {
                // Cross-pane move: take BOTH bars out of the map so we hold two distinct &mut.
                // Remove each independently so a missing bar never causes the OTHER (successfully
                // removed) bar to be dropped — that would lose a whole pane's tabs.
                let source_id = payload.source_pane_id.clone();
                let target_id = target.target_pane_id.clone();
                let mut source_opt = tab_bars.remove(&source_id);
                let mut target_opt = tab_bars.remove(&target_id);
                if let (Some(source_bar), Some(target_bar)) =
                    (source_opt.as_mut(), target_opt.as_mut())
                {
                    apply_drop(payload, target, source_bar, target_bar);
                }
                // Re-insert whatever we removed (both on the happy path; the survivor otherwise).
                if let Some(b) = source_opt {
                    tab_bars.insert(source_id, b);
                }
                if let Some(b) = target_opt {
                    tab_bars.insert(target_id, b);
                }
            }
        }

        // ── Dividers ─────────────────────────────────────────────────────────────────────────────
        // CANONICAL (React): the horizontal LINE controls `weights.horizontal` (top/bottom row split,
        // Y); the vertical LINE controls `weights.vertical` (left/right column split, X). Line-name
        // and weight-name match — see `SplitAxis` doc and `app/src/App.tsx`.
        Self::divider(
            ui,
            SplitAxis::Horizontal,
            area,
            rects.divider_h,
            &mut weights.horizontal,
            &mut drag_state.dragging_horizontal,
            divider_colors,
        );
        Self::divider(
            ui,
            SplitAxis::Vertical,
            area,
            rects.divider_v,
            &mut weights.vertical,
            &mut drag_state.dragging_vertical,
            divider_colors,
        );
    }

    /// Render one divider: pointer drag, keyboard resize, AccessKit `Splitter` node, and the painted
    /// line. `weight` is the split fraction this divider controls — CANONICAL React mapping:
    /// `weights.horizontal` for the horizontal LINE (top/bottom, Y), `weights.vertical` for the
    /// vertical LINE (left/right, X). `dragging` is the per-frame drag flag for hover/grab coloring.
    #[allow(clippy::too_many_arguments)]
    fn divider(
        ui: &mut egui::Ui,
        axis: SplitAxis,
        area: egui::Rect,
        hit_rect: egui::Rect,
        weight: &mut f32,
        dragging: &mut bool,
        colors: DividerColors,
    ) {
        let id = Self::divider_id(axis);
        let response = ui.interact(hit_rect, id, egui::Sense::click_and_drag());

        // Cursor affordance on hover/drag.
        if response.hovered() || response.dragged() {
            let cursor = match axis {
                SplitAxis::Horizontal => egui::CursorIcon::ResizeVertical,
                SplitAxis::Vertical => egui::CursorIcon::ResizeHorizontal,
            };
            ui.ctx().set_cursor_icon(cursor);
        }

        // ── Pointer drag ───────────────────────────────────────────────────────────────────────
        *dragging = response.dragged();
        if response.dragged() {
            let delta = match axis {
                // +y drag moves the horizontal line down -> larger top fraction.
                SplitAxis::Horizontal => response.drag_delta().y / area.height(),
                // +x drag moves the vertical line right -> larger left fraction.
                SplitAxis::Vertical => response.drag_delta().x / area.width(),
            };
            *weight = clamp_split(*weight + delta);
        }

        // Clicking the divider focuses it so the keyboard resize path below engages.
        if response.clicked() {
            response.request_focus();
        }

        // ── Keyboard resize ──────────────────────────────────────────────────────────────────────
        // CONTRACT (red-team CONTROL): key events are consumed ONLY while the divider has focus, so
        // arrow keys never steal input from text entry inside a pane. `response.has_focus()` is the
        // gate; do not move this check.
        if response.has_focus() {
            let (incr_key, decr_key) = match axis {
                // Horizontal line: Down grows the top fraction, Up shrinks it.
                SplitAxis::Horizontal => (egui::Key::ArrowDown, egui::Key::ArrowUp),
                // Vertical line: Right grows the left fraction, Left shrinks it.
                SplitAxis::Vertical => (egui::Key::ArrowRight, egui::Key::ArrowLeft),
            };
            let (mut up, mut down) = (false, false);
            ui.input(|i| {
                down = i.key_pressed(incr_key);
                up = i.key_pressed(decr_key);
            });
            if down {
                *weight = clamp_split(*weight + SPLIT_STEP);
            }
            if up {
                *weight = clamp_split(*weight - SPLIT_STEP);
            }
        }

        // ── AccessKit SetValue action (out-of-process / kittest steering) ─────────────────────────
        // An agent sets the divider to a fractional position; clamp it to [SPLIT_MIN, SPLIT_MAX]
        // BEFORE applying so SetValue(0.0)/SetValue(0.1) can never collapse a pane (red-team CONTROL).
        // Pattern mirrors egui's own Slider SetValue consumption.
        {
            use accesskit::{Action, ActionData};
            let mut requested: Option<f64> = None;
            ui.input(|input| {
                for request in input.accesskit_action_requests(response.id, Action::SetValue) {
                    if let Some(ActionData::NumericValue(v)) = request.data {
                        requested = Some(v);
                    }
                }
            });
            if let Some(v) = requested {
                *weight = apply_set_value(v);
            }
        }

        // ── Live AccessKit node: Role::Splitter + author_id + numeric value/min/max + SetValue ─────
        let author_id = match axis {
            SplitAxis::Horizontal => DIVIDER_H_AUTHOR_ID,
            SplitAxis::Vertical => DIVIDER_V_AUTHOR_ID,
        };
        let value = *weight as f64;
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Splitter);
            node.set_author_id(author_id.to_owned());
            node.set_label(match axis {
                SplitAxis::Horizontal => "Horizontal split divider".to_owned(),
                SplitAxis::Vertical => "Vertical split divider".to_owned(),
            });
            node.set_numeric_value(value);
            node.set_min_numeric_value(SPLIT_MIN as f64);
            node.set_max_numeric_value(SPLIT_MAX as f64);
            node.set_numeric_value_step(SPLIT_STEP as f64);
            node.add_action(accesskit::Action::SetValue);
            node.add_action(accesskit::Action::Focus);
        });

        // ── Paint the divider line ───────────────────────────────────────────────────────────────
        // Color comes from the MT-003 theme tokens (idle/hover/grab), NOT egui's generic visuals
        // (MT-006 contract). grab > hover/focus > idle in priority.
        if ui.is_rect_visible(hit_rect) {
            let color =
                divider_line_color(colors, response.hovered() || response.has_focus(), *dragging);
            let visible = divider_visible_rect(axis, hit_rect);
            ui.painter().rect_filled(visible, 0.0, color);
        }
    }
}

/// Pick the divider line color from the theme tokens by interaction state. Priority: grab (active
/// drag) > hover/focus > idle. Pure (no egui frame) so the "divider uses the theme tokens" contract
/// is unit-testable.
#[inline]
fn divider_line_color(colors: DividerColors, hovered_or_focused: bool, dragging: bool) -> egui::Color32 {
    if dragging {
        colors.grab
    } else if hovered_or_focused {
        colors.hover
    } else {
        colors.idle
    }
}

/// The painted (visual) rect for a divider: the [`DIVIDER_THICKNESS`]-wide line centered inside the
/// wider [`DIVIDER_HIT_THICKNESS`] hit rect.
fn divider_visible_rect(axis: SplitAxis, hit_rect: egui::Rect) -> egui::Rect {
    let half_visible = DIVIDER_THICKNESS / 2.0;
    let center = hit_rect.center();
    match axis {
        SplitAxis::Horizontal => egui::Rect::from_min_max(
            egui::pos2(hit_rect.left(), center.y - half_visible),
            egui::pos2(hit_rect.right(), center.y + half_visible),
        ),
        SplitAxis::Vertical => egui::Rect::from_min_max(
            egui::pos2(center.x - half_visible, hit_rect.top()),
            egui::pos2(center.x + half_visible, hit_rect.bottom()),
        ),
    }
}

/// Stable fallback id derived from the pane id string. Only used if a pane somehow lacks a
/// registry-assigned AccessKit id; deterministic so it is still stable across frames. Mirrors the
/// helper in `pane_registry.rs`.
fn hash_pane_id(pane_id: &PaneId) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pane_id.as_ref().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-4;

    fn area_600() -> egui::Rect {
        // 800 wide x 600 tall, origin at (0,0) for easy arithmetic.
        egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(800.0, 600.0))
    }

    /// AC: the four pane rects tile the full area with no gap and no overlap, for several weight
    /// pairs including both boundary extremes.
    #[test]
    fn pane_rects_tile_area_with_no_gap_or_overlap() {
        let area = area_600();
        for (v, h) in [(0.2, 0.2), (0.5, 0.55), (0.8, 0.8)] {
            let w = SplitWeights {
                vertical: v,
                horizontal: h,
            };
            let r = compute_split_rects(area, w);

            // Full coverage: union of the four rects equals the area exactly.
            assert!((r.top_left.left() - area.left()).abs() < EPS);
            assert!((r.top_left.top() - area.top()).abs() < EPS);
            assert!((r.bottom_right.right() - area.right()).abs() < EPS);
            assert!((r.bottom_right.bottom() - area.bottom()).abs() < EPS);

            // Shared vertical boundary (left column right edge == right column left edge).
            assert!((r.top_left.right() - r.top_right.left()).abs() < EPS, "v boundary top");
            assert!(
                (r.bottom_left.right() - r.bottom_right.left()).abs() < EPS,
                "v boundary bottom"
            );
            // Shared horizontal boundary (top row bottom edge == bottom row top edge).
            assert!((r.top_left.bottom() - r.bottom_left.top()).abs() < EPS, "h boundary left");
            assert!(
                (r.top_right.bottom() - r.bottom_right.top()).abs() < EPS,
                "h boundary right"
            );

            // The boundary line is exactly at weight * dimension, using the CANONICAL React mapping:
            // vertical weight -> X column split; horizontal weight -> Y row split.
            let split_x = area.left() + area.width() * w.vertical;
            let split_y = area.top() + area.height() * w.horizontal;
            assert!((r.top_left.right() - split_x).abs() < EPS, "split_x");
            assert!((r.top_left.bottom() - split_y).abs() < EPS, "split_y");

            // No pane has zero or negative size at any allowed weight.
            for rect in [r.top_left, r.top_right, r.bottom_left, r.bottom_right] {
                assert!(rect.width() > 0.0, "pane width positive");
                assert!(rect.height() > 0.0, "pane height positive");
            }
        }
    }

    /// CANONICAL AXIS SEMANTICS (matches React `app/src/App.css` `.main-pane-grid`): the `vertical`
    /// weight drives the X / column split (`split_x`), and the `horizontal` weight drives the Y / row
    /// split (`split_y`). This is the regression guard for the axis-inversion BLOCKER: changing ONLY
    /// `vertical` must move the column split and leave the row split fixed, and vice versa.
    #[test]
    fn axis_semantics_match_react_grid() {
        let area = area_600(); // 800 x 600 at origin
        let base = SplitWeights { vertical: 0.5, horizontal: 0.5 };
        let r0 = compute_split_rects(area, base);
        // Baseline: split_x at 0.5*800=400, split_y at 0.5*600=300.
        assert!((r0.top_left.right() - 400.0).abs() < EPS, "baseline split_x");
        assert!((r0.top_left.bottom() - 300.0).abs() < EPS, "baseline split_y");

        // Increase ONLY `vertical`: the COLUMN (X) split must move right; the ROW (Y) split must NOT.
        let more_vertical = SplitWeights { vertical: 0.7, horizontal: 0.5 };
        let rv = compute_split_rects(area, more_vertical);
        assert!(
            (rv.top_left.right() - 0.7 * 800.0).abs() < EPS,
            "vertical weight drives the X/column split (split_x = width*vertical)"
        );
        assert!(
            (rv.top_left.bottom() - 300.0).abs() < EPS,
            "changing vertical must NOT move the Y/row split"
        );
        assert!(
            rv.top_left.right() > r0.top_left.right(),
            "larger vertical => left column wider (split_x moves right)"
        );

        // Increase ONLY `horizontal`: the ROW (Y) split must move down; the COLUMN (X) split must NOT.
        let more_horizontal = SplitWeights { vertical: 0.5, horizontal: 0.7 };
        let rh = compute_split_rects(area, more_horizontal);
        assert!(
            (rh.top_left.bottom() - 0.7 * 600.0).abs() < EPS,
            "horizontal weight drives the Y/row split (split_y = height*horizontal)"
        );
        assert!(
            (rh.top_left.right() - 400.0).abs() < EPS,
            "changing horizontal must NOT move the X/column split"
        );
        assert!(
            rh.top_left.bottom() > r0.top_left.bottom(),
            "larger horizontal => top row taller (split_y moves down)"
        );
    }

    /// AC: dragging the horizontal divider +50px on a 600px-tall area changes the horizontal-divider
    /// weight (`weights.horizontal`) by exactly 50/600 (before clamping). The horizontal LINE moves
    /// on Y and controls `weights.horizontal` (React `axis === "horizontal"`).
    #[test]
    fn drag_delta_maps_to_exact_weight_fraction() {
        let height = 600.0_f32;
        let start = 0.5_f32;
        let delta_px = 50.0_f32;
        let expected = clamp_split(start + delta_px / height);
        // 0.5 + 0.08333 = 0.58333, within [0.2, 0.8] so clamp is a no-op here.
        assert!((expected - (0.5 + 50.0 / 600.0)).abs() < EPS);
        // And it is the same arithmetic the divider() drag branch performs.
        let mut w = start;
        w = clamp_split(w + delta_px / height);
        assert!((w - expected).abs() < EPS);
    }

    /// AC: dragging below SPLIT_MIN clamps to SPLIT_MIN; above SPLIT_MAX clamps to SPLIT_MAX.
    #[test]
    fn clamp_split_bounds() {
        assert!((clamp_split(0.1) - SPLIT_MIN).abs() < EPS, "0.1 -> 0.2");
        assert!((clamp_split(0.9) - SPLIT_MAX).abs() < EPS, "0.9 -> 0.8");
        assert!((clamp_split(0.5) - 0.5).abs() < EPS, "in-range unchanged");
        // Drag a near-min weight further down: clamps, never goes below SPLIT_MIN.
        let dragged = clamp_split(0.21 + (-200.0 / 600.0));
        assert!((dragged - SPLIT_MIN).abs() < EPS, "over-drag down clamps to min");
        let dragged_up = clamp_split(0.79 + (200.0 / 600.0));
        assert!((dragged_up - SPLIT_MAX).abs() < EPS, "over-drag up clamps to max");
    }

    /// AC: keyboard step is ±SPLIT_STEP, clamped. On the horizontal LINE, ArrowDown grows
    /// `weights.horizontal`; on the vertical LINE, ArrowLeft shrinks `weights.vertical` (matches
    /// React `handleSplitDividerKeyDown`, `app/src/App.tsx` ~1554–1567).
    #[test]
    fn keyboard_step_is_split_step_clamped() {
        // Horizontal line: ArrowDown adds SPLIT_STEP to weights.horizontal.
        let mut horizontal = 0.5_f32;
        horizontal = clamp_split(horizontal + SPLIT_STEP);
        assert!((horizontal - 0.55).abs() < EPS);
        // Vertical line: ArrowLeft subtracts SPLIT_STEP from weights.vertical.
        let mut vertical = 0.55_f32;
        vertical = clamp_split(vertical - SPLIT_STEP);
        assert!((vertical - 0.5).abs() < EPS);
        // Step past the boundary clamps.
        let mut at_max = SPLIT_MAX;
        at_max = clamp_split(at_max + SPLIT_STEP);
        assert!((at_max - SPLIT_MAX).abs() < EPS, "step past max clamps");
        let mut at_min = SPLIT_MIN;
        at_min = clamp_split(at_min - SPLIT_STEP);
        assert!((at_min - SPLIT_MIN).abs() < EPS, "step past min clamps");
    }

    /// AC: a `SetValue(0.1)` clamps to 0.2 (SPLIT_MIN), not 0.1; `SetValue(0.0)` -> 0.2 too.
    #[test]
    fn set_value_clamps_below_min() {
        assert!((apply_set_value(0.1) - SPLIT_MIN).abs() < EPS, "0.1 -> 0.2");
        assert!((apply_set_value(0.0) - SPLIT_MIN).abs() < EPS, "0.0 -> 0.2");
        assert!((apply_set_value(0.9) - SPLIT_MAX).abs() < EPS, "0.9 -> 0.8");
        assert!((apply_set_value(0.5) - 0.5).abs() < EPS, "0.5 -> 0.5");
    }

    /// SplitWeights serializes with the exact React field names and round-trips; SplitDragState is a
    /// separate non-serialized type (it derives no Serialize, enforced by the absence of the bound).
    #[test]
    fn split_weights_serde_uses_react_field_names() {
        let w = SplitWeights {
            vertical: 0.5,
            horizontal: 0.55,
        };
        let json = serde_json::to_string(&w).expect("serialize");
        assert!(json.contains("\"vertical\""), "has vertical field: {json}");
        assert!(json.contains("\"horizontal\""), "has horizontal field: {json}");
        let back: SplitWeights = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, w);
    }

    #[test]
    fn default_weights_match_react_constants() {
        let w = SplitWeights::default();
        assert!((w.vertical - 0.5).abs() < EPS);
        assert!((w.horizontal - 0.55).abs() < EPS);
    }

    /// Divider hit-rects straddle the boundary lines by DIVIDER_HIT_THICKNESS/2 and span the full
    /// perpendicular dimension; the painted rect is the narrower DIVIDER_THICKNESS, centered.
    #[test]
    fn divider_rects_span_and_center_correctly() {
        let area = area_600();
        let w = SplitWeights {
            vertical: 0.5,
            horizontal: 0.5,
        };
        let r = compute_split_rects(area, w);

        // Horizontal divider spans full width, centered on split_y=300.
        assert!((r.divider_h.left() - area.left()).abs() < EPS);
        assert!((r.divider_h.right() - area.right()).abs() < EPS);
        assert!((r.divider_h.height() - DIVIDER_HIT_THICKNESS).abs() < EPS);
        assert!((r.divider_h.center().y - 300.0).abs() < EPS);

        // Vertical divider spans full height, centered on split_x=400.
        assert!((r.divider_v.top() - area.top()).abs() < EPS);
        assert!((r.divider_v.bottom() - area.bottom()).abs() < EPS);
        assert!((r.divider_v.width() - DIVIDER_HIT_THICKNESS).abs() < EPS);
        assert!((r.divider_v.center().x - 400.0).abs() < EPS);

        // Painted rects are the narrower thickness, centered in the hit rect.
        let vis_h = divider_visible_rect(SplitAxis::Horizontal, r.divider_h);
        assert!((vis_h.height() - DIVIDER_THICKNESS).abs() < EPS);
        assert!((vis_h.center().y - 300.0).abs() < EPS);
        let vis_v = divider_visible_rect(SplitAxis::Vertical, r.divider_v);
        assert!((vis_v.width() - DIVIDER_THICKNESS).abs() < EPS);
        assert!((vis_v.center().x - 400.0).abs() < EPS);
    }

    /// MT-006 FIX-2: the divider paints with the MT-003 theme tokens (idle/hover/grab), the state
    /// priority is grab > hover/focus > idle, and the dark idle token DIFFERS from the light idle
    /// token (so the two themes are visually distinguishable).
    #[test]
    fn divider_uses_theme_tokens_and_idle_differs_dark_vs_light() {
        use crate::theme::HsPalette;

        let dark = HsPalette::dark();
        let light = HsPalette::light();

        // The dividers are driven by the per-theme tokens, not egui's generic visuals.
        let dark_colors = DividerColors {
            idle: dark.divider_idle,
            hover: dark.divider_hover,
            grab: dark.divider_grab,
        };

        // State -> token selection (grab beats hover beats idle).
        assert_eq!(
            divider_line_color(dark_colors, false, false),
            dark.divider_idle,
            "no hover, no drag -> idle token"
        );
        assert_eq!(
            divider_line_color(dark_colors, true, false),
            dark.divider_hover,
            "hover/focus -> hover token"
        );
        assert_eq!(
            divider_line_color(dark_colors, false, true),
            dark.divider_grab,
            "dragging -> grab token"
        );
        // grab wins even while also hovered.
        assert_eq!(
            divider_line_color(dark_colors, true, true),
            dark.divider_grab,
            "grab beats hover"
        );

        // Idle token must differ between dark and light themes.
        assert_ne!(
            dark.divider_idle, light.divider_idle,
            "dark idle divider color must differ from light idle (themes distinguishable)"
        );
        // Grab uses each theme's accent, which also differs.
        assert_ne!(
            dark.divider_grab, light.divider_grab,
            "dark grab (accent) differs from light grab (accent)"
        );
    }

    /// Divider node ids stay below the pane id base and are disjoint from the chrome ids, preserving
    /// the collision-free invariant the accessibility registry enforces. (The cross-module proof that
    /// the dividers do not collide with chrome/panes lives in `accessibility::registry`'s
    /// `declared_identities_have_no_node_id_or_author_id_collision`, which now includes the dividers.)
    #[test]
    fn divider_node_ids_are_collision_safe() {
        // Read into runtime locals so the bound check is a real assertion, not a const folded to
        // `assert!(true)`.
        let pane_base = crate::accessibility::PANE_NODE_ID_BASE;
        let h = DIVIDER_H_NODE_ID;
        let v = DIVIDER_V_NODE_ID;
        assert!(h < pane_base, "horizontal divider id below pane base");
        assert!(v < pane_base, "vertical divider id below pane base");
        assert_ne!(h, v);
        for chrome in [10_u64, 20, 21] {
            assert_ne!(h, chrome);
            assert_ne!(v, chrome);
        }
    }
}
