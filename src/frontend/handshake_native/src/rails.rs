//! Integrated splitter + scrollbar rails for the native work-surface shell (WP-KERNEL-011 MT-010).
//!
//! ## What this provides
//!
//! A single, cohesive "rail" visual style shared by BOTH kinds of draggable thin track in the shell:
//!
//! - the split dividers between panes (MT-006 [`crate::split_layout::SplitLayoutWidget`]), and
//! - scrollbars inside a pane that overflows its viewport.
//!
//! Both now paint through the SAME [`RailWidget`] primitive with the SAME four interaction states
//! ([`RailState`]), the SAME dimensions ([`RailDimensions`]: a thin 4px visual strip centered inside
//! a wider 8px pointer hit-target), and the SAME theme-token colors ([`RailColors`]). The result is
//! the "integrated rail" look the C2 GUI contract asks for: a divider and a scrollbar read as the
//! same family of control, not two unrelated widgets.
//!
//! ## Colors come from the live theme, never from a static constructor
//!
//! [`RailColors::from_palette`] derives the four state colors from the active MT-003
//! [`crate::theme::HsPalette`] EVERY frame (the caller passes the live palette). This is the
//! red-team control for "rails stay dark after switching to light theme": because the colors are
//! re-derived from the live tokens each frame, a runtime theme toggle is reflected on the very next
//! frame. The dark/light palettes deliberately differ (see `theme/palette.rs`), so the two themes
//! are visually distinguishable.
//!
//! ## Why a shared primitive instead of two ad-hoc widgets
//!
//! MT-006 already hand-rolled the divider's hit/visual rect split and state coloring inline in
//! `split_layout.rs`. MT-010's job is to lift that into ONE reusable rail so the scrollbar and the
//! divider cannot drift apart visually, and so a single dimension/color change updates both. The
//! divider keeps its existing `Role::Splitter` AccessKit node + clamp semantics (MT-006 contract);
//! MT-010 only routes its PAINTING through the shared rail. The scrollbar adds a `Role::ScrollBar`
//! node so an out-of-process model can drive it by `SetValue`.
//!
//! ## egui scrollbar default style override
//!
//! egui does not expose a hook to swap the scrollbar RENDERER, so [`apply_rail_scrollbar_style`]
//! overrides the relevant `style.spacing.scroll` dimensions (bar width = the rail hit thickness,
//! handle min length = the rail min thumb) and the widget bg fills so egui's OWN `ScrollArea`
//! scrollbars pick up the rail dimensions + colors automatically — without replacing every
//! `ScrollArea` call site. For a call site that needs the exact 4px-visual/8px-hit rail rendering
//! (e.g. a future custom editor pane), [`ScrollbarRail`] is provided to render the rail explicitly.

use egui::accesskit;

use crate::theme::HsPalette;

/// Fixed AccessKit/egui `NodeId`s for the four default panes' vertical scrollbar rails (MT-010).
///
/// These occupy a FRESH id band (40..43) that is disjoint from every other declared identity:
/// theme toggle (10), chrome (20/21), dividers (30/31), tab bars (60..63), merge-back (64..67), and
/// the pane id space (>= 100). The collision test in `accessibility::registry` proves the
/// disjointness across the whole declared set. The order matches the default pane seed
/// (`pane-a`..`pane-d`).
///
/// A scrollbar rail uses a fixed-value `egui::Id` (`from_high_entropy_bits`) so its AccessKit
/// `NodeId` is stable across frames and process restarts — the same convention the divider, chrome,
/// and toggle use — which is what lets an out-of-process model address the scrollbar by a stable id.
pub const SCROLLBAR_V_NODE_IDS: [(&str, u64); 4] = [
    ("scrollbar-v-pane-a", 40),
    ("scrollbar-v-pane-b", 41),
    ("scrollbar-v-pane-c", 42),
    ("scrollbar-v-pane-d", 43),
];

/// The fixed `egui::Id` for a scrollbar rail given its fixed `NodeId`.
///
/// # Safety
/// `from_high_entropy_bits` assumes a well-distributed value for egui's `IdMap`; a single
/// hand-assigned, never-reused fixed id is safe (it cannot self-collide), matching the divider /
/// chrome / toggle convention.
pub fn scrollbar_rail_id(node_id: u64) -> egui::Id {
    unsafe { egui::Id::from_high_entropy_bits(node_id) }
}

/// The four interaction states a rail can be in. A divider is never [`RailState::Disabled`] (it is
/// always draggable); a scrollbar is [`RailState::Disabled`] when its content fits the viewport
/// (nothing to scroll), in which case it paints a dim track and senses no drag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailState {
    /// At rest: thin, barely-visible.
    Idle,
    /// Pointer is over the rail (or it has keyboard focus): brighter affordance.
    Hover,
    /// Actively being dragged: accent color.
    Grab,
    /// Non-interactive (e.g. a scrollbar whose content fits): dimmest, no thumb.
    Disabled,
}

/// The four theme-token colors a rail paints with, one per [`RailState`]. Derived from the active
/// [`HsPalette`] via [`RailColors::from_palette`] each frame so a runtime theme toggle is reflected
/// immediately. `dark_theme()` / `light_theme()` are convenience constructors that read the
/// corresponding base palette; production wiring uses `from_palette` with the app's live palette so
/// per-workspace token overrides are honored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RailColors {
    pub idle: egui::Color32,
    pub hover: egui::Color32,
    pub grab: egui::Color32,
    pub disabled: egui::Color32,
}

impl RailColors {
    /// Pick the color for an interaction state.
    pub fn for_state(&self, state: RailState) -> egui::Color32 {
        match state {
            RailState::Idle => self.idle,
            RailState::Hover => self.hover,
            RailState::Grab => self.grab,
            RailState::Disabled => self.disabled,
        }
    }

    /// Derive the rail colors from a live theme palette (the production path). Reads the MT-003
    /// `scrollbar_*` rail tokens, which carry the exact dark/light idle/hover/grab/disabled spec from
    /// the MT-010 design table. Because the caller passes the LIVE palette every frame, a runtime
    /// theme toggle (or a per-workspace token override) updates the rails on the next frame.
    pub fn from_palette(palette: &HsPalette) -> Self {
        Self {
            idle: palette.scrollbar_idle,
            hover: palette.scrollbar_hover,
            grab: palette.scrollbar_grab,
            disabled: palette.scrollbar_disabled,
        }
    }

    /// The dark-theme rail palette (convenience; equals `from_palette(&HsPalette::dark())`).
    pub fn dark_theme() -> Self {
        Self::from_palette(&HsPalette::dark())
    }

    /// The light-theme rail palette (convenience; equals `from_palette(&HsPalette::light())`).
    pub fn light_theme() -> Self {
        Self::from_palette(&HsPalette::light())
    }
}

/// Rail geometry: a thin [`Self::visual_thickness`] painted strip centered inside a wider
/// [`Self::hit_thickness`] pointer hit-target, with [`Self::corner_radius`] rounded ends and a
/// [`Self::min_thumb_length`] floor on a scrollbar thumb. Values match the MT-010 design table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RailDimensions {
    /// Painted strip thickness (the visible line). 4px.
    pub visual_thickness: f32,
    /// Pointer hit-target thickness (centered over the visual strip). 8px — twice the visual so the
    /// rail is easy to grab without looking thick.
    pub hit_thickness: f32,
    /// Rounded-end corner radius. 2px.
    pub corner_radius: f32,
    /// Minimum scrollbar thumb length so a tiny thumb never becomes ungrabbable. 20px.
    pub min_thumb_length: f32,
}

impl Default for RailDimensions {
    fn default() -> Self {
        Self {
            visual_thickness: 4.0,
            hit_thickness: 8.0,
            corner_radius: 2.0,
            min_thumb_length: 20.0,
        }
    }
}

/// Which way a rail runs. A horizontal divider line and a vertical scrollbar both have an
/// orientation; the rail math (which dimension is the thin one vs the long one) keys off this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailOrientation {
    /// The rail runs horizontally (its long axis is X; it is thin in Y). A horizontal split divider
    /// or a horizontal scrollbar.
    Horizontal,
    /// The rail runs vertically (its long axis is Y; it is thin in X). A vertical split divider or a
    /// vertical scrollbar.
    Vertical,
}

/// The shared low-level rail painter. Given a hit-rect and an orientation, it computes the centered
/// thin visual rect and paints it in the state color. Both [`SplitterRail`] and [`ScrollbarRail`]
/// use this so a divider and a scrollbar thumb render identically.
pub struct RailWidget;

impl RailWidget {
    /// The painted (visual) rect: the [`RailDimensions::visual_thickness`]-thin strip centered inside
    /// the wider `hit_rect`, spanning the rail's full long axis. This is the geometry that makes the
    /// rail read as a thin 4px line while still being an 8px grab target.
    pub fn visual_rect(
        orientation: RailOrientation,
        hit_rect: egui::Rect,
        dims: RailDimensions,
    ) -> egui::Rect {
        let half_visible = dims.visual_thickness / 2.0;
        let center = hit_rect.center();
        match orientation {
            RailOrientation::Horizontal => egui::Rect::from_min_max(
                egui::pos2(hit_rect.left(), center.y - half_visible),
                egui::pos2(hit_rect.right(), center.y + half_visible),
            ),
            RailOrientation::Vertical => egui::Rect::from_min_max(
                egui::pos2(center.x - half_visible, hit_rect.top()),
                egui::pos2(center.x + half_visible, hit_rect.bottom()),
            ),
        }
    }

    /// Paint the thin visual strip for a rail segment in the given state color, with rounded ends.
    /// The caller is responsible for sensing/interaction; this only draws.
    pub fn paint(
        ui: &egui::Ui,
        orientation: RailOrientation,
        hit_rect: egui::Rect,
        state: RailState,
        colors: RailColors,
        dims: RailDimensions,
    ) {
        if !ui.is_rect_visible(hit_rect) {
            return;
        }
        let visual = Self::visual_rect(orientation, hit_rect, dims);
        ui.painter()
            .rect_filled(visual, dims.corner_radius, colors.for_state(state));
    }
}

/// Map an interaction response to a rail state for a DIVIDER (which is never disabled): grab while
/// dragged, hover while hovered or focused, else idle. Pure so the state-selection contract is
/// unit-testable without an egui frame.
#[inline]
pub fn divider_rail_state(hovered_or_focused: bool, dragging: bool) -> RailState {
    if dragging {
        RailState::Grab
    } else if hovered_or_focused {
        RailState::Hover
    } else {
        RailState::Idle
    }
}

// ── Splitter rail ───────────────────────────────────────────────────────────────────────────────

/// A split divider rendered in the integrated rail style. MT-006 owns the divider's interaction,
/// clamp, and `Role::Splitter` AccessKit node; MT-010's [`SplitterRail`] is the PAINT primitive the
/// divider now routes through, so the divider and the scrollbar share one visual. The caller passes
/// the divider's already-computed hit-rect and current state, and `paint` draws the thin rail strip.
pub struct SplitterRail;

impl SplitterRail {
    /// Paint a divider's rail strip. `hovered_or_focused` / `dragging` come from the divider's egui
    /// response (MT-006 `split_layout::SplitLayoutWidget::divider`). The painted strip is the thin
    /// [`RailDimensions::visual_thickness`] line centered in the divider's `hit_rect`.
    pub fn paint(
        ui: &egui::Ui,
        orientation: RailOrientation,
        hit_rect: egui::Rect,
        hovered_or_focused: bool,
        dragging: bool,
        colors: RailColors,
        dims: RailDimensions,
    ) {
        let state = divider_rail_state(hovered_or_focused, dragging);
        RailWidget::paint(ui, orientation, hit_rect, state, colors, dims);
    }
}

// ── Scrollbar rail ──────────────────────────────────────────────────────────────────────────────

/// Pure scrollbar thumb geometry, computed from the content/viewport sizes and the scroll offset.
/// Split out as a plain struct so the thumb math (the part with the off-by-one / overflow / div-by-0
/// red-team risks) is fully unit-testable without an egui frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarThumb {
    /// Length of the thumb along the track's long axis.
    pub thumb_length: f32,
    /// Offset of the thumb's near edge from the track's near edge along the long axis.
    pub thumb_pos: f32,
    /// Whether the content overflows the viewport (i.e. there is anything to scroll). When `false`
    /// the rail is [`RailState::Disabled`] and no thumb is drawn.
    pub scrollable: bool,
}

impl ScrollbarThumb {
    /// Compute the thumb geometry.
    ///
    /// - `content_size`: total length of the scrollable content along the axis.
    /// - `viewport_size`: visible length of the viewport along the axis.
    /// - `scroll_offset`: current scroll position (0 = top/left), clamped to `[0, content-viewport]`.
    /// - `track_length`: length of the scrollbar track along the axis.
    /// - `dims`: rail dimensions (for the min thumb length).
    ///
    /// thumb_length = clamp(viewport/content * track, min_thumb, track); thumb_pos =
    /// offset/(content-viewport) * (track-thumb_length), clamped so `thumb_pos + thumb_length <=
    /// track_length` for ALL inputs (the property the red-team min-control asserts). When the content
    /// fits (`content <= viewport`) the rail is not scrollable and the thumb math is skipped entirely
    /// — this is the guard against dividing by `content - viewport == 0`.
    pub fn compute(
        content_size: f32,
        viewport_size: f32,
        scroll_offset: f32,
        track_length: f32,
        dims: RailDimensions,
    ) -> Self {
        // Content fits (or track is degenerate): nothing to scroll. Skip the thumb math BEFORE any
        // division by (content - viewport), which would be zero/negative here.
        if content_size <= viewport_size || track_length <= 0.0 {
            return Self {
                thumb_length: 0.0,
                thumb_pos: 0.0,
                scrollable: false,
            };
        }

        let raw_len = (viewport_size / content_size) * track_length;
        // Floor at min_thumb (but never exceed the track itself).
        let thumb_length = raw_len.clamp(dims.min_thumb_length.min(track_length), track_length);

        let max_offset = content_size - viewport_size; // > 0 here
        let travel = (track_length - thumb_length).max(0.0);
        let clamped_offset = scroll_offset.clamp(0.0, max_offset);
        let thumb_pos = if max_offset > 0.0 {
            (clamped_offset / max_offset) * travel
        } else {
            0.0
        };
        // Defensive: keep the thumb fully inside the track even with float rounding.
        let thumb_pos = thumb_pos.clamp(0.0, travel);

        Self {
            thumb_length,
            thumb_pos,
            scrollable: true,
        }
    }

    /// The scroll offset that corresponds to a thumb position, the inverse of [`Self::compute`].
    /// Used by the drag handler: a thumb dragged by `delta` px maps back to an offset delta of
    /// `delta * (content - viewport) / (track - thumb_length)`.
    pub fn offset_for_thumb_delta(
        delta: f32,
        content_size: f32,
        viewport_size: f32,
        track_length: f32,
        thumb_length: f32,
    ) -> f32 {
        let max_offset = content_size - viewport_size;
        let travel = track_length - thumb_length;
        if max_offset <= 0.0 || travel <= 0.0 {
            return 0.0;
        }
        delta * (max_offset / travel)
    }
}

/// A scrollbar rendered in the integrated rail style, with a `Role::ScrollBar` AccessKit node so an
/// out-of-process model can drive it (the contract's parallel-agent requirement). Provided for call
/// sites that need the exact 4px-visual/8px-hit rail rendering; the global
/// [`apply_rail_scrollbar_style`] handles egui's built-in `ScrollArea` scrollbars for everything
/// else.
///
/// The caller owns the scroll offset (the rail is stateless): `show` returns the NEW offset the
/// caller should apply to its content. AccessKit `SetValue(fraction)` and `ScrollUp`/`ScrollDown`
/// actions are consumed here and folded into the returned offset, so an agent action and a pointer
/// drag share one offset path.
pub struct ScrollbarRail {
    /// Fixed `egui::Id` for the rail (so its AccessKit `NodeId` is stable for steering).
    pub id: egui::Id,
    /// Which way the scrollbar runs.
    pub orientation: RailOrientation,
    /// The full track rect the scrollbar occupies.
    pub track_rect: egui::Rect,
    /// Total length of the scrollable content along the axis.
    pub content_size: f32,
    /// Visible length of the viewport along the axis.
    pub viewport_size: f32,
    /// Current scroll offset (0 = top/left).
    pub scroll_offset: f32,
    /// Rail colors (derived from the live theme by the caller).
    pub colors: RailColors,
    /// Rail dimensions.
    pub dims: RailDimensions,
    /// Stable out-of-process match key (e.g. `"scrollbar-v-pane-a"`).
    pub author_id: String,
    /// How far one `ScrollUp`/`ScrollDown` action moves the offset (a "line" step). 40px by default
    /// matches egui's wheel-ish line height feel.
    pub line_step: f32,
}

/// The outcome of rendering a [`ScrollbarRail`] for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollbarRailResponse {
    /// The scroll offset after this frame's interaction (pointer drag or AccessKit action). The
    /// caller applies this to its content. Equal to the input offset when nothing moved (including
    /// the disabled / content-fits case).
    pub new_offset: f32,
    /// The pointer is over the thumb this frame.
    pub hovered: bool,
    /// The thumb is being dragged this frame.
    pub dragged: bool,
    /// The rail's interaction state (drives the painted color). [`RailState::Disabled`] when the
    /// content fits the viewport.
    pub state: RailState,
}

impl ScrollbarRail {
    /// The track's long-axis length.
    fn track_length(&self) -> f32 {
        match self.orientation {
            RailOrientation::Horizontal => self.track_rect.width(),
            RailOrientation::Vertical => self.track_rect.height(),
        }
    }

    /// The thumb's hit-rect for a given thumb geometry: the thin rail strip at `thumb_pos` along the
    /// track's long axis, spanning the rail's [`RailDimensions::hit_thickness`] across.
    fn thumb_rect(&self, thumb: ScrollbarThumb) -> egui::Rect {
        match self.orientation {
            RailOrientation::Vertical => {
                let top = self.track_rect.top() + thumb.thumb_pos;
                egui::Rect::from_min_max(
                    egui::pos2(self.track_rect.left(), top),
                    egui::pos2(self.track_rect.right(), top + thumb.thumb_length),
                )
            }
            RailOrientation::Horizontal => {
                let left = self.track_rect.left() + thumb.thumb_pos;
                egui::Rect::from_min_max(
                    egui::pos2(left, self.track_rect.top()),
                    egui::pos2(left + thumb.thumb_length, self.track_rect.bottom()),
                )
            }
        }
    }

    /// Render the scrollbar rail for one frame and return the (possibly updated) scroll offset.
    ///
    /// Renders the dim Idle track, then (if scrollable) the thumb in its interaction-state color,
    /// senses a pointer drag on the thumb, consumes any AccessKit `SetValue` / `ScrollUp` /
    /// `ScrollDown` action, and emits the live `Role::ScrollBar` node (value = offset fraction,
    /// `SetValue` + scroll actions) so an out-of-process model can drive it.
    pub fn show(&self, ui: &mut egui::Ui) -> ScrollbarRailResponse {
        let max_offset = (self.content_size - self.viewport_size).max(0.0);
        let track_length = self.track_length();
        let thumb = ScrollbarThumb::compute(
            self.content_size,
            self.viewport_size,
            self.scroll_offset,
            track_length,
            self.dims,
        );

        // ── Content fits: disabled rail, dim track, no thumb, no sensing, offset unchanged ─────────
        if !thumb.scrollable {
            RailWidget::paint(
                ui,
                self.orientation,
                self.track_rect,
                RailState::Disabled,
                self.colors,
                self.dims,
            );
            // Still emit a live node so the rail is addressable; value 0, no scroll actions (nothing
            // to scroll). Role::ScrollBar with the stable author_id.
            self.emit_node(ui, 0.0, false);
            return ScrollbarRailResponse {
                new_offset: self.scroll_offset,
                hovered: false,
                dragged: false,
                state: RailState::Disabled,
            };
        }

        // ── Paint the dim track behind the thumb ──────────────────────────────────────────────────
        RailWidget::paint(
            ui,
            self.orientation,
            self.track_rect,
            RailState::Idle,
            self.colors,
            self.dims,
        );

        // ── Consume AccessKit actions BEFORE ui.interact (out-of-process / kittest steering) ───────
        // ORDER IS LOAD-BEARING: egui's `Context::interact_with_*` auto-consumes `ScrollUp`/
        // `ScrollDown`/`ScrollIntoView` action requests targeting a Response's id (it routes them into
        // its own built-in scroll-area delta — see egui `context.rs` `consume_accesskit_action_requests`).
        // The rail owns its OWN offset (it is not inside an egui ScrollArea), so we must read those
        // actions from the input queue HERE, before `ui.interact(self.id, ...)` removes them, or
        // ScrollUp/ScrollDown would be swallowed and never reach the rail. SetValue is not auto-
        // consumed by egui, but we read it here too so all three actions share one code path.
        let mut new_offset = self.consume_accesskit_actions(ui, self.id, self.scroll_offset, max_offset);

        // ── Sense a drag on the thumb ─────────────────────────────────────────────────────────────
        let thumb_rect = self.thumb_rect(thumb);
        let response = ui.interact(thumb_rect, self.id, egui::Sense::click_and_drag());

        if response.dragged() {
            let delta = match self.orientation {
                RailOrientation::Vertical => response.drag_delta().y,
                RailOrientation::Horizontal => response.drag_delta().x,
            };
            let offset_delta = ScrollbarThumb::offset_for_thumb_delta(
                delta,
                self.content_size,
                self.viewport_size,
                track_length,
                thumb.thumb_length,
            );
            new_offset = (new_offset + offset_delta).clamp(0.0, max_offset);
        }

        let state = divider_rail_state(response.hovered(), response.dragged());

        // ── Paint the thumb in its state color ────────────────────────────────────────────────────
        // Recompute the thumb rect at the (possibly) new offset so the thumb tracks the drag this
        // same frame.
        let painted_thumb = ScrollbarThumb::compute(
            self.content_size,
            self.viewport_size,
            new_offset,
            track_length,
            self.dims,
        );
        RailWidget::paint(
            ui,
            self.orientation,
            self.thumb_rect(painted_thumb),
            state,
            self.colors,
            self.dims,
        );

        // ── Live AccessKit node ───────────────────────────────────────────────────────────────────
        let fraction = if max_offset > 0.0 {
            (new_offset / max_offset) as f64
        } else {
            0.0
        };
        self.emit_node(ui, fraction, true);

        ScrollbarRailResponse {
            new_offset,
            hovered: response.hovered(),
            dragged: response.dragged(),
            state,
        }
    }

    /// Consume any pending AccessKit action for this rail and fold it into the offset.
    ///
    /// - `SetValue(fraction)`: set the offset to `fraction * max_offset` (0..1 -> top..bottom).
    /// - `ScrollDown` / `ScrollUp`: move the offset by one [`Self::line_step`], clamped.
    fn consume_accesskit_actions(
        &self,
        ui: &egui::Ui,
        node_id: egui::Id,
        current: f32,
        max_offset: f32,
    ) -> f32 {
        use accesskit::{Action, ActionData};
        let mut offset = current;
        ui.input(|input| {
            for request in input.accesskit_action_requests(node_id, Action::SetValue) {
                if let Some(ActionData::NumericValue(v)) = request.data {
                    let frac = (v as f32).clamp(0.0, 1.0);
                    offset = (frac * max_offset).clamp(0.0, max_offset);
                }
            }
            for _ in input.accesskit_action_requests(node_id, Action::ScrollDown) {
                offset = (offset + self.line_step).clamp(0.0, max_offset);
            }
            for _ in input.accesskit_action_requests(node_id, Action::ScrollUp) {
                offset = (offset - self.line_step).clamp(0.0, max_offset);
            }
        });
        offset
    }

    /// Emit the live `Role::ScrollBar` node for this rail: stable author_id, the offset fraction as
    /// the numeric value (0..1), and the `SetValue` / scroll actions an out-of-process model uses.
    /// When `scrollable` is false the scroll actions are omitted (nothing to scroll) but the node is
    /// still emitted so the rail is addressable.
    fn emit_node(&self, ui: &egui::Ui, fraction: f64, scrollable: bool) {
        let author_id = self.author_id.clone();
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_role(accesskit::Role::ScrollBar);
            node.set_author_id(author_id);
            node.set_label(match self.orientation {
                RailOrientation::Vertical => "Vertical scrollbar".to_owned(),
                RailOrientation::Horizontal => "Horizontal scrollbar".to_owned(),
            });
            node.set_numeric_value(fraction);
            node.set_min_numeric_value(0.0);
            node.set_max_numeric_value(1.0);
            node.add_action(accesskit::Action::SetValue);
            if scrollable {
                node.add_action(accesskit::Action::ScrollUp);
                node.add_action(accesskit::Action::ScrollDown);
            }
        });
    }
}

// ── Global egui scrollbar style override ──────────────────────────────────────────────────────────

/// Override egui's built-in `ScrollArea` scrollbar style so every scroll area in the shell renders in
/// the rail dimensions + colors, WITHOUT replacing each `ScrollArea` call site with [`ScrollbarRail`].
///
/// egui has no hook to swap the scrollbar renderer, so we override:
/// - `spacing.scroll.bar_width` = the rail hit thickness (8px) — egui 0.33 keeps scrollbar width on
///   `Spacing::scroll` (`ScrollStyle::bar_width`), NOT on `Visuals` as older egui did. (The MT-010
///   contract snippet referenced `visuals.scroll_bar_width`, which does not exist in the pinned egui
///   0.33 family; the equivalent is `spacing.scroll.bar_width`.)
/// - `spacing.scroll.handle_min_length` = the rail min thumb (20px),
/// - `spacing.scroll.bar_inner_margin` = 2px (the rail's breathing room),
/// - the widget bg fills (`inactive`/`hovered`/`active`) = the rail idle/hover/grab colors so the
///   egui handle picks up the rail palette,
/// - `floating = false` so the bar reserves space and reads as an integrated rail rather than an
///   overlay.
///
/// RED-TEAM CONTROL: this must NOT touch `panel_fill` or `window_fill` (which would recolor every
/// panel/window background, not just scrollbars). It also leaves `extreme_bg_color` alone — that is
/// the TextEdit/code background, and recoloring it to the rail idle color would wreck editor
/// contrast. Only scrollbar-specific dimensions and the interactive widget handle fills change.
pub fn apply_rail_scrollbar_style(ctx: &egui::Context, colors: RailColors, dims: RailDimensions) {
    ctx.style_mut(|style| {
        style.spacing.scroll.floating = false;
        style.spacing.scroll.bar_width = dims.hit_thickness;
        style.spacing.scroll.handle_min_length = dims.min_thumb_length;
        style.spacing.scroll.bar_inner_margin = 2.0;

        // The egui scroll HANDLE is drawn with the interactive widget bg fills; point them at the
        // rail palette so egui's own scrollbar handle matches the rail. The handle is dim at rest,
        // brighter on hover, accent while grabbed — the same state progression the rail uses.
        style.visuals.widgets.inactive.bg_fill = colors.idle;
        style.visuals.widgets.hovered.bg_fill = colors.hover;
        style.visuals.widgets.active.bg_fill = colors.grab;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    const EPS: f32 = 1e-3;

    // ── Color state selection (AC-1) ─────────────────────────────────────────────────────────────

    /// AC: dark idle is #2A2A2F at alpha ~153; dark grab is #7A7AFF fully opaque. Colors are stored
    /// premultiplied (egui), so we compare the UNMULTIPLIED RGBA the spec is written in.
    #[test]
    fn dark_rail_colors_match_spec() {
        let c = RailColors::dark_theme();

        // RGB channels asserted within ±1 of spec to absorb the premultiplied-alpha round-trip (egui
        // stores premultiplied; unmultiplying re-introduces up to 1 LSB at non-opaque alpha).
        assert_rgb_within_1(c.for_state(RailState::Idle), [0x2A, 0x2A, 0x2F], "dark idle");
        let idle = c.for_state(RailState::Idle).to_srgba_unmultiplied();
        assert!((idle[3] as i32 - 153).abs() <= 1, "dark idle alpha ~153 (0.6); got {}", idle[3]);

        // Grab is fully opaque, so it round-trips exactly.
        let grab = c.for_state(RailState::Grab).to_srgba_unmultiplied();
        assert_eq!(grab[0], 0x7A, "dark grab R");
        assert_eq!(grab[1], 0x7A, "dark grab G");
        assert_eq!(grab[2], 0xFF, "dark grab B");
        assert_eq!(grab[3], 255, "dark grab fully opaque");

        assert_rgb_within_1(c.for_state(RailState::Hover), [0x4A, 0x4A, 0x55], "dark hover");
        let hover = c.for_state(RailState::Hover).to_srgba_unmultiplied();
        assert!((hover[3] as i32 - 217).abs() <= 1, "dark hover alpha ~217 (0.85)");

        // Disabled is a low-alpha (0.3 -> 77/255) token stored premultiplied; unmultiplying back at
        // low alpha loses up to 1 LSB per channel (the documented egui premultiplied round-trip, see
        // theme/palette.rs). Assert each channel within ±1 of the #1E1E22 spec.
        assert_rgb_within_1(c.for_state(RailState::Disabled), [0x1E, 0x1E, 0x22], "dark disabled");
        let disabled = c.for_state(RailState::Disabled).to_srgba_unmultiplied();
        assert!((disabled[3] as i32 - 77).abs() <= 1, "dark disabled alpha ~77 (0.3)");
    }

    /// AC: light palette matches the light spec, and differs from dark (themes distinguishable).
    /// Assert a premultiplied `Color32`'s unmultiplied RGB is within ±1 of the spec per channel
    /// (absorbs the egui premultiplied-alpha round-trip at non-opaque alpha).
    fn assert_rgb_within_1(color: egui::Color32, want: [u8; 3], what: &str) {
        let got = color.to_srgba_unmultiplied();
        for (i, ch) in ["R", "G", "B"].iter().enumerate() {
            assert!(
                (got[i] as i32 - want[i] as i32).abs() <= 1,
                "{what} {ch} within ±1 of {:#X}; got {:#X}",
                want[i],
                got[i]
            );
        }
    }

    #[test]
    fn light_rail_colors_match_spec_and_differ_from_dark() {
        let c = RailColors::light_theme();
        assert_rgb_within_1(c.for_state(RailState::Idle), [0xC8, 0xC8, 0xD0], "light idle");
        let grab = c.for_state(RailState::Grab).to_srgba_unmultiplied();
        assert_eq!([grab[0], grab[1], grab[2]], [0x50, 0x50, 0xFF], "light grab rgb");
        assert_eq!(grab[3], 255, "light grab opaque");

        // Themes must be distinguishable on every state.
        let d = RailColors::dark_theme();
        assert_ne!(c.idle, d.idle, "idle differs dark vs light");
        assert_ne!(c.hover, d.hover, "hover differs");
        assert_ne!(c.grab, d.grab, "grab differs");
        assert_ne!(c.disabled, d.disabled, "disabled differs");
    }

    /// `from_palette` derives from the live tokens, so a different palette yields different colors —
    /// the property that makes a runtime theme toggle update the rails on the next frame.
    #[test]
    fn from_palette_reflects_live_theme() {
        let dark = RailColors::from_palette(&HsPalette::dark());
        let light = RailColors::from_palette(&HsPalette::light());
        assert_eq!(dark, RailColors::dark_theme());
        assert_eq!(light, RailColors::light_theme());
        assert_ne!(dark, light);
    }

    // ── Thumb geometry (AC-2, AC-3) ───────────────────────────────────────────────────────────────

    /// AC: content 1000, viewport 200, offset 0, track 200 -> thumb_length = max(20, 200/1000*200) =
    /// 40px, thumb_pos = 0.
    #[test]
    fn thumb_length_and_pos_at_top() {
        let t = ScrollbarThumb::compute(1000.0, 200.0, 0.0, 200.0, RailDimensions::default());
        assert!(t.scrollable, "content overflows -> scrollable");
        assert!((t.thumb_length - 40.0).abs() < EPS, "thumb_length 40px; got {}", t.thumb_length);
        assert!((t.thumb_pos - 0.0).abs() < EPS, "thumb_pos 0 at offset 0; got {}", t.thumb_pos);
    }

    /// AC: content fits the viewport -> Disabled, no movement, thumb not scrollable.
    #[test]
    fn content_fits_is_disabled_and_no_thumb() {
        // content == viewport (boundary) and content < viewport both fit.
        for content in [200.0_f32, 150.0] {
            let t = ScrollbarThumb::compute(content, 200.0, 0.0, 200.0, RailDimensions::default());
            assert!(!t.scrollable, "content {content} fits -> not scrollable");
            assert!((t.thumb_length).abs() < EPS, "no thumb when content fits");
            assert!((t.thumb_pos).abs() < EPS);
        }
    }

    /// RED-TEAM min-control: content == viewport exactly must not panic and the rail is Disabled with
    /// new_offset == scroll_offset (proven via offset inverse returning 0 movement).
    #[test]
    fn content_exactly_fits_no_panic_no_movement() {
        let t = ScrollbarThumb::compute(200.0, 200.0, 0.0, 200.0, RailDimensions::default());
        assert!(!t.scrollable);
        // offset inverse with no overflow yields no movement.
        let d = ScrollbarThumb::offset_for_thumb_delta(50.0, 200.0, 200.0, 200.0, 0.0);
        assert!((d).abs() < EPS, "no offset movement when content fits");
    }

    /// RED-TEAM min-control (property): thumb_pos + thumb_length <= track_length for ALL valid
    /// inputs, so the thumb never visually overflows the track lane.
    #[test]
    fn thumb_never_overflows_track_property() {
        let dims = RailDimensions::default();
        // Deterministic pseudo-random sweep (no rand dep): vary content/viewport/offset/track.
        let mut seed: u64 = 0x9E3779B97F4A7C15;
        let mut next = || {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((seed >> 33) as f32) / (u32::MAX as f32)
        };
        for _ in 0..5000 {
            let viewport = 10.0 + next() * 500.0;
            let content = viewport + next() * 2000.0; // always >= viewport (may be tiny overflow)
            let track = 10.0 + next() * 600.0;
            let max_off = (content - viewport).max(0.0);
            let offset = next() * max_off * 1.5; // intentionally allow over-range; compute clamps it
            let t = ScrollbarThumb::compute(content, viewport, offset, track, dims);
            if t.scrollable {
                assert!(
                    t.thumb_pos + t.thumb_length <= track + 1e-2,
                    "thumb overflows track: pos={} len={} track={} (content={} viewport={} off={})",
                    t.thumb_pos, t.thumb_length, track, content, viewport, offset
                );
                assert!(t.thumb_pos >= -1e-3, "thumb_pos non-negative");
                assert!(t.thumb_length >= 0.0);
            }
        }
    }

    /// Thumb sits at the bottom when offset is at max: thumb_pos + thumb_length == track_length.
    #[test]
    fn thumb_at_bottom_fills_to_track_end() {
        let dims = RailDimensions::default();
        let (content, viewport, track) = (1000.0_f32, 200.0_f32, 200.0_f32);
        let max_off = content - viewport;
        let t = ScrollbarThumb::compute(content, viewport, max_off, track, dims);
        assert!(t.scrollable);
        assert!(
            (t.thumb_pos + t.thumb_length - track).abs() < 1e-2,
            "thumb bottom edge at track end: pos={} len={} track={}",
            t.thumb_pos, t.thumb_length, track
        );
    }

    /// min_thumb_length floor: a huge content gives a tiny proportional thumb that is floored at 20px.
    #[test]
    fn tiny_thumb_floored_at_min() {
        let dims = RailDimensions::default();
        // viewport/content = 1/100 -> raw thumb = 2px on a 200px track; floored to 20px.
        let t = ScrollbarThumb::compute(20000.0, 200.0, 0.0, 200.0, dims);
        assert!((t.thumb_length - dims.min_thumb_length).abs() < EPS, "floored to 20px; got {}", t.thumb_length);
    }

    // ── Visual vs hit rect (AC-8) ─────────────────────────────────────────────────────────────────

    /// AC: the rail fills the hit target (8px) while painting only the centered visual strip (4px).
    #[test]
    fn visual_rect_is_thin_strip_centered_in_hit_rect() {
        let dims = RailDimensions::default();
        // Vertical rail: 8px-wide hit rect, 100px tall.
        let hit = egui::Rect::from_min_max(egui::pos2(100.0, 0.0), egui::pos2(108.0, 100.0));
        let vis = RailWidget::visual_rect(RailOrientation::Vertical, hit, dims);
        assert!((vis.width() - dims.visual_thickness).abs() < EPS, "visual is 4px wide; got {}", vis.width());
        assert!((hit.width() - dims.hit_thickness).abs() < EPS, "hit is 8px wide");
        assert!((vis.center().x - hit.center().x).abs() < EPS, "visual centered in hit (x)");
        assert!((vis.height() - hit.height()).abs() < EPS, "visual spans full long axis");

        // Horizontal rail: 8px-tall hit rect, 100px wide.
        let hith = egui::Rect::from_min_max(egui::pos2(0.0, 50.0), egui::pos2(100.0, 58.0));
        let vish = RailWidget::visual_rect(RailOrientation::Horizontal, hith, dims);
        assert!((vish.height() - dims.visual_thickness).abs() < EPS, "visual is 4px tall");
        assert!((vish.center().y - hith.center().y).abs() < EPS, "visual centered in hit (y)");
        assert!((vish.width() - hith.width()).abs() < EPS, "visual spans full long axis");
    }

    // ── Divider state selection (AC-5 maps to weight; state here) ──────────────────────────────────

    #[test]
    fn divider_state_priority_grab_beats_hover_beats_idle() {
        assert_eq!(divider_rail_state(false, false), RailState::Idle);
        assert_eq!(divider_rail_state(true, false), RailState::Hover);
        assert_eq!(divider_rail_state(false, true), RailState::Grab);
        assert_eq!(divider_rail_state(true, true), RailState::Grab, "grab beats hover");
    }

    /// Offset inverse: a thumb dragged by `delta` maps back to the expected offset delta.
    #[test]
    fn offset_for_thumb_delta_is_inverse_of_pos() {
        // content 1000, viewport 200, track 200, thumb 40 -> travel 160, max_off 800.
        // dragging the thumb the full travel (160px) must move the offset the full max (800).
        let d = ScrollbarThumb::offset_for_thumb_delta(160.0, 1000.0, 200.0, 200.0, 40.0);
        assert!((d - 800.0).abs() < 1e-1, "full-travel drag -> full offset; got {d}");
        // half travel -> half offset.
        let h = ScrollbarThumb::offset_for_thumb_delta(80.0, 1000.0, 200.0, 200.0, 40.0);
        assert!((h - 400.0).abs() < 1e-1, "half-travel drag -> half offset; got {h}");
    }
}
