//! Pop-out window + merge-back for the native work surface (WP-KERNEL-011 MT-008).
//!
//! ## What this provides
//!
//! Any pane in the 2x2 split (MT-006) can be **popped out** into its own OS-native window and later
//! **merged back** into the main layout. egui's multi-viewport mechanism
//! ([`egui::Context::show_viewport_immediate`] + [`egui::ViewportBuilder`]) is the native windowing
//! path — no external windowing crate is added. The *immediate* (not deferred) viewport is required
//! because the pane content is borrowed host state (registry + factories + tab state) that a
//! `Send + Sync + 'static` deferred callback cannot reach; see [`PopOutManager::show_all`]. While a
//! pane is popped out:
//!
//! - its record STAYS in the [`crate::pane_registry::PaneRegistry`] (the single source of truth is
//!   never fragmented — only the pane's *render destination* changes);
//! - the split layout renders a [`PopOutPlaceholder`] tile in the pane's grid rect, showing the
//!   surface label "<label> (popped out)" and a "Merge Back" button;
//! - the detached window hosts the pane's tab bar + body, so the pane is fully usable on a second
//!   monitor and remains accessible (its own AccessKit tree, see [`PopOutManager::show_all`]).
//!
//! Merge-back can be triggered three ways, all converging on a single state flag:
//! 1. the placeholder tile's "Merge Back" button (handled by the app -> [`PopOutManager::merge_back`]);
//! 2. the detached window's OS close button ([`egui::ViewportInfo::close_requested`] inside the
//!    immediate viewport callback -> [`PopOutState::open`] set to `false`);
//! 3. an out-of-process model issuing an AccessKit `Click` on the merge-back button.
//!
//! After [`show_all`](PopOutManager::show_all) renders the open pop-outs for the frame, every entry
//! whose `open == false` is drained. The pane is already in the registry, so the placeholder tile
//! disappears next frame because [`is_popped_out`](PopOutManager::is_popped_out) now returns false.
//!
//! ## Headless testability (honest scope)
//!
//! egui spawns a *real* OS window from `show_viewport_immediate` only when the backend reports
//! multi-viewport support ([`egui::Context::embed_viewports`] == `false`, which eframe sets on the
//! wgpu/winit backend). On a plain headless `egui::Context` (egui_kittest) `embed_viewports` is
//! `true`, so the immediate callback runs **embedded** in the current frame instead of opening a
//! window. Either way the SAME callback (and therefore the SAME `render_content` host path) draws the
//! pop-out's tab bar, body, and AccessKit nodes, so the popped-out content is drivable headlessly and
//! identical to what a real second window shows. The genuine OS-window behavior — a separate
//! top-level window, focus management, and the native title-bar close button click — still cannot be
//! exercised without a real winit event loop.
//!
//! To keep merge-back honest and headlessly provable, the OS close-button path routes
//! `close_requested()` (inside the immediate callback) into [`PopOutState::request_close`]. Tests
//! drive the app's REAL `popout_manager` through its update loop and assert the app's own
//! `is_popped_out` flips, AND drive the placeholder Merge Back button via a live AccessKit `Click`
//! (the in-process path). The remaining "the OS actually raised a second window and the user clicked
//! its native X" step is the only part left to manual/integration verification; it is documented, not
//! faked.
//!
//! ## Red-team controls (from the MT-008 contract)
//!
//! - **One viewport per frame**: `show_viewport_immediate` is called exactly once per *open* pop-out,
//!   driven by the `pop_outs` map, so no per-frame window storm (RISK: viewport-per-frame).
//! - **No focus theft (HBR-QUIET)**: [`ViewportBuilder::with_active(false)`] keeps the OS from
//!   raising the detached window to the foreground on creation.
//! - **No double-removal panic**: draining happens AFTER `show_all`, never during iteration, and the
//!   drain is keyed so the same entry cannot be removed twice in one frame.
//! - **Off-screen geometry**: [`PopOutGeometry::clamped_to`] resets an out-of-bounds restored
//!   position back into the primary screen rect (MT-009 restore safety).

use std::collections::HashMap;

use egui::accesskit;
use serde::{Deserialize, Serialize};

use crate::pane_registry::PaneId;

/// Fixed AccessKit/egui `NodeId` band base for the per-pane "Merge Back" placeholder buttons. The
/// four fixed grid panes (pane-a..pane-d) map to 64..67 by their spatial slot — a fresh band placed
/// directly above the MT-007 tab-bar container band (60..63) and strictly below the pane id base
/// (100), so the collision-free invariant in `accessibility::registry` holds. Declared in
/// [`crate::accessibility::registry::DECLARED_IDENTITIES`] so the collision test covers them.
pub const MERGE_BACK_NODE_ID_BASE: u64 = 64;

/// The four fixed pane slots paired with their merge-back button `NodeId`, in the same spatial order
/// as `split_layout`'s 2x2 grid and `tab_bar::TABBAR_SLOTS`. A dynamic pane added in a later MT has
/// no fixed slot and would fall back to a hashed id (see [`merge_back_egui_id`]); the four seeded
/// panes are the addressable-by-fixed-id set the acceptance criteria assert by string.
pub const MERGE_BACK_SLOTS: [(&str, u64); 4] = [
    ("pane-a", MERGE_BACK_NODE_ID_BASE),     // 64
    ("pane-b", MERGE_BACK_NODE_ID_BASE + 1), // 65
    ("pane-c", MERGE_BACK_NODE_ID_BASE + 2), // 66
    ("pane-d", MERGE_BACK_NODE_ID_BASE + 3), // 67
];

/// The fixed merge-back button `NodeId` for a pane slot, if it is one of the four grid panes.
pub fn merge_back_node_id(pane_id: &str) -> Option<u64> {
    MERGE_BACK_SLOTS
        .iter()
        .find(|(slot, _)| *slot == pane_id)
        .map(|(_, id)| *id)
}

/// Stable out-of-process author_id for a pane's "Merge Back" button (`merge-back-{pane_id}`).
pub fn merge_back_author_id(pane_id: &str) -> String {
    format!("merge-back-{pane_id}")
}

/// Stable out-of-process author_id for a detached pop-out window's root (`popout-window-{pane_id}`).
pub fn popout_window_author_id(pane_id: &str) -> String {
    format!("popout-window-{pane_id}")
}

/// Stable `egui::Id` for a pane's merge-back button. For the four fixed grid panes this is the
/// fixed-value id (so its AccessKit `NodeId` equals [`merge_back_node_id`]); for any other pane it is
/// derived from the author_id string so it is still stable across frames. Mirrors
/// [`crate::tab_bar::tabbar_egui_id`].
pub fn merge_back_egui_id(pane_id: &str) -> egui::Id {
    match merge_back_node_id(pane_id) {
        // # Safety: a single hand-assigned, never-reused fixed id (64..67) cannot self-collide;
        // entropy only affects egui's child IdMap distribution. The band is disjoint from all other
        // declared ids by construction (see MERGE_BACK_NODE_ID_BASE doc).
        Some(node_id) => unsafe { egui::Id::from_high_entropy_bits(node_id) },
        None => egui::Id::new(merge_back_author_id(pane_id)),
    }
}

/// Default detached-window size in logical points when a pane is first popped out.
pub const DEFAULT_POPOUT_SIZE: egui::Vec2 = egui::vec2(800.0, 600.0);
/// Fallback position used when a restored pop-out geometry would land off-screen.
pub const FALLBACK_POPOUT_POS: egui::Pos2 = egui::pos2(100.0, 100.0);

/// On-screen geometry of a detached pop-out window. Derives `Serialize`/`Deserialize` so MT-009 can
/// persist pop-out positions in the layout snapshot and reopen windows where the operator left them.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PopOutGeometry {
    /// Top-left of the detached window on screen.
    pub pos: egui::Pos2,
    /// Size of the detached window.
    pub size: egui::Vec2,
}

impl PopOutGeometry {
    /// Geometry at `pos` with the default pop-out size.
    pub fn at(pos: egui::Pos2) -> Self {
        Self {
            pos,
            size: DEFAULT_POPOUT_SIZE,
        }
    }

    /// Return a copy whose `pos` is guaranteed to be inside `extent`. If the top-left is outside the
    /// bounds — e.g. a snapshot saved on a multi-monitor setup whose secondary monitor is now
    /// disconnected — reset it to [`FALLBACK_POPOUT_POS`] so the window cannot open off-screen and
    /// become unreachable (MT-008 red-team CONTROL: off-screen geometry). The size is left untouched.
    ///
    /// `extent` MUST be the FULL virtual-desktop / all-monitors bounding rect, NOT the primary
    /// window's `content_rect()` — a pop-out the operator dragged onto a second monitor is legitimately
    /// outside the primary content rect and must be preserved. This is a RESTORE-time safety net only
    /// (MT-009 applies it once when reopening a saved layout); a LIVE pop-out's geometry is never
    /// clamped (see [`PopOutManager::show_all`]).
    pub fn clamped_to(self, extent: egui::Rect) -> Self {
        if extent.contains(self.pos) {
            self
        } else {
            Self {
                pos: FALLBACK_POPOUT_POS,
                size: self.size,
            }
        }
    }
}

impl Default for PopOutGeometry {
    fn default() -> Self {
        Self::at(FALLBACK_POPOUT_POS)
    }
}

/// One popped-out pane's live state. The pane itself stays in the registry; this only records that
/// the pane currently renders into a detached viewport and where that window sits.
#[derive(Debug, Clone)]
pub struct PopOutState {
    /// The pane this window hosts. Its record lives in the registry, never here.
    pub pane_id: PaneId,
    /// egui's deferred viewport id for this pop-out, derived stably from the pane id so the same pane
    /// always reuses the same OS window handle across pop-out operations.
    pub viewport_id: egui::ViewportId,
    /// Where the detached window sits on screen.
    pub geometry: PopOutGeometry,
    /// `true` while the window is open; set `false` to request merge-back this frame.
    pub open: bool,
}

impl PopOutState {
    /// Build the pop-out state for `pane_id`, deriving a stable [`egui::ViewportId`] from the pane id
    /// string so the OS window handle is repeatable across pop-out/merge-back cycles.
    pub fn new(pane_id: PaneId, geometry: PopOutGeometry) -> Self {
        let viewport_id = egui::ViewportId::from_hash_of(pane_id.as_ref());
        Self {
            pane_id,
            viewport_id,
            geometry,
            open: true,
        }
    }

    /// Request that this pop-out merge back (close). Idempotent: calling it twice is harmless, which
    /// is what prevents the double-removal panic when both the OS close button and the placeholder
    /// Merge Back button fire in the same frame (MT-008 red-team CONTROL: registry double-removal).
    /// This is the exact seam the immediate viewport callback hits on `close_requested()`; see the
    /// module-level "Headless testability" note.
    pub fn request_close(&mut self) {
        self.open = false;
    }
}

/// Owns every active pop-out. Keyed by `PaneId` so a pane has at most one detached window, which is
/// also what makes `show_viewport_deferred` idempotent (one call per open pop-out per frame).
#[derive(Debug, Default)]
pub struct PopOutManager {
    pop_outs: HashMap<PaneId, PopOutState>,
}

impl PopOutManager {
    pub fn new() -> Self {
        Self {
            pop_outs: HashMap::new(),
        }
    }

    /// Pop a pane out into its own window. Inserts (or replaces) a [`PopOutState`] with `open == true`.
    /// Does NOT touch the registry — the pane record is unchanged; only its render destination moves.
    /// Re-popping an already-open pane refreshes its geometry and re-opens it.
    pub fn pop_out(&mut self, pane_id: PaneId, geometry: PopOutGeometry) {
        let state = PopOutState::new(pane_id.clone(), geometry);
        self.pop_outs.insert(pane_id, state);
    }

    /// Request merge-back for a pane: marks its pop-out `open = false`. The entry is removed by the
    /// next [`show_all`](Self::show_all) drain. A no-op if the pane is not popped out.
    pub fn merge_back(&mut self, pane_id: &PaneId) {
        if let Some(state) = self.pop_outs.get_mut(pane_id) {
            state.request_close();
        }
    }

    /// Drive the OS-close-button seam for a pane's pop-out: marks it `open = false`, exactly as
    /// [`show_all`](Self::show_all) does when the immediate viewport reports
    /// `ViewportInfo::close_requested`. Mechanically the same as [`merge_back`](Self::merge_back) (the
    /// OS close button and the Merge Back button converge on the one `open` flag), but named for the
    /// OS-close path so tests/drivers can simulate the native close without a winit window. Returns
    /// `true` if a pop-out existed for `pane_id`.
    pub fn request_os_close(&mut self, pane_id: &PaneId) -> bool {
        match self.pop_outs.get_mut(pane_id) {
            Some(state) => {
                state.request_close();
                true
            }
            None => false,
        }
    }

    /// Whether a pane currently renders into a detached window. The split layout uses this to decide
    /// whether to draw the pane body or the [`PopOutPlaceholder`] tile.
    pub fn is_popped_out(&self, pane_id: &PaneId) -> bool {
        self.pop_outs.contains_key(pane_id)
    }

    /// Number of active pop-outs (test/diagnostic visibility).
    pub fn len(&self) -> usize {
        self.pop_outs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.pop_outs.is_empty()
    }

    /// Read-only access to a pane's pop-out state (tests / MT-009 snapshot wiring).
    pub fn get(&self, pane_id: &PaneId) -> Option<&PopOutState> {
        self.pop_outs.get(pane_id)
    }

    /// Pane ids of all active pop-outs, in stable sorted order (so snapshots and tests are
    /// deterministic regardless of `HashMap` iteration order).
    pub fn popped_out_ids(&self) -> Vec<PaneId> {
        let mut ids: Vec<PaneId> = self.pop_outs.keys().cloned().collect();
        ids.sort();
        ids
    }

    /// Render every open pop-out into its own native viewport, then drain the ones that requested
    /// close this frame.
    ///
    /// For each `open` pop-out this calls [`egui::Context::show_viewport_immediate`] exactly once
    /// (idempotency comes from iterating the map, so no per-frame window storm) with a
    /// [`ViewportBuilder`] that:
    /// - titles the window `"Handshake – <pane_type_label>"`;
    /// - positions/sizes it from the geometry;
    /// - shows the OS close button;
    /// - **does not** steal focus on creation (`with_active(false)`, HBR-QUIET).
    ///
    /// ## Why `show_viewport_immediate`, not `show_viewport_deferred`
    ///
    /// The pop-out's rich content (tab bar + factory body + the pane's live AccessKit node) is
    /// borrowed state owned by the host (`PaneRegistry`, the factory map, per-pane tab state). A
    /// `show_viewport_deferred` callback must be `Send + Sync + 'static`, so it **cannot** reach that
    /// borrowed state — the deferred path could only draw an empty `CentralPanel`, leaving the real
    /// detached window BLANK on the wgpu/winit backend (where `embed_viewports() == false`). The
    /// immediate callback is `impl FnMut(&Context, ViewportClass)` with no `'static`/`Send` bound, so
    /// it borrows `render_content` (and through it the registry/factories/tab state) directly and
    /// draws the SAME content on BOTH paths:
    /// - headless (`embed_viewports() == true`, egui_kittest): egui runs the callback inline THIS
    ///   frame with [`ViewportClass::Embedded`], so the popped-out pane's content + AccessKit nodes
    ///   land in the live tree the test reads;
    /// - real window (`embed_viewports() == false`, eframe): egui spawns the second OS window and runs
    ///   the SAME callback inside it with [`ViewportClass::Immediate`], so the pane is fully usable on
    ///   a second monitor.
    ///
    /// One closure, one source of truth, no `embed_viewports()` gate. The "immediate viewport blocks
    /// the main thread" tradeoff is acceptable for a handful of detached panes and is the only way to
    /// render borrowed host state into a native child window each frame.
    ///
    /// `render_content` draws the pane's tab bar + body + live AccessKit node inside the viewport's
    /// `CentralPanel`; it receives the `ViewportClass` so the caller can branch on embedded vs native.
    /// This method itself draws the window-root `Window` AccessKit node (so the detached window is
    /// addressable as `popout-window-{pane_id}`) and wires `close_requested()` -> merge-back.
    ///
    /// After all open pop-outs are shown, entries with `open == false` are drained (collect keys
    /// first, then remove — never mutate the map during iteration). Returns the pane ids that were
    /// merged back this frame so the caller can react (e.g. log / refocus).
    ///
    /// `render_content` signature: `(ctx, class, pane_id) -> ()`. The pane is still in the registry,
    /// so the caller looks it up by `pane_id` and renders it through the SAME factory + tab-bar path
    /// the main window uses, keeping one source of truth.
    ///
    /// `title_for` resolves the detached-window title for a pane. The host wires it to the registry's
    /// `PaneType::label()` via [`popout_title_for`] so the window title is the contract-required
    /// `"Handshake – <pane_type_label>"` (e.g. `"Handshake – Workspace"`). The window title is also
    /// the label of the pop-out's AccessKit root `Window` node.
    ///
    /// ## Geometry is NOT clamped here
    ///
    /// `show_all` deliberately does **not** clamp or overwrite a pop-out's `geometry` against the
    /// primary window's `content_rect()` each frame. Doing so would snap a window the operator dragged
    /// onto a second monitor (legitimately outside the primary content rect) back to the fallback
    /// position every frame. The off-screen safety net is [`PopOutGeometry::clamped_to`], which MT-009
    /// applies ONCE at restore time against the full virtual-desktop / monitor extent — not the
    /// primary content rect — and only when a saved position is provably outside all monitors. While a
    /// pop-out is live its geometry is left exactly as set.
    pub fn show_all<R, T>(
        &mut self,
        ctx: &egui::Context,
        title_for: T,
        mut render_content: R,
    ) -> Vec<PaneId>
    where
        R: FnMut(&egui::Context, egui::ViewportClass, &PaneId),
        T: Fn(&PaneId) -> String,
    {
        // Show each OPEN pop-out exactly once. We iterate a cloned key list so the immediate callback
        // can borrow `render_content` (and through it the host's registry/factories/tab state) without
        // also holding a borrow of `self.pop_outs`.
        let open_ids: Vec<PaneId> = self
            .pop_outs
            .iter()
            .filter(|(_, s)| s.open)
            .map(|(id, _)| id.clone())
            .collect();

        for pane_id in &open_ids {
            // Geometry is used as-is (NOT clamped against the primary content rect — see method doc:
            // that would snap a second-monitor window back every frame).
            let (viewport_id, title, geometry) = {
                let state = &self.pop_outs[pane_id];
                (state.viewport_id, title_for(pane_id), state.geometry)
            };

            let builder = egui::ViewportBuilder::default()
                .with_title(title.clone())
                .with_position(geometry.pos)
                .with_inner_size(geometry.size)
                .with_close_button(true)
                // HBR-QUIET: do NOT raise the detached window to the foreground / steal focus on
                // creation. Without this, Windows CreateWindowEx can foreground the new window.
                .with_active(false);

            // Track whether the OS close button was hit so we can request merge-back after the call.
            // The immediate callback borrows (not 'static), so a plain local flag is enough — no need
            // to round-trip through egui shared memory.
            let mut close_requested = false;
            ctx.show_viewport_immediate(viewport_id, builder, |ctx, class| {
                // OS close-button request: route it back to the parent frame, which calls
                // request_close() after this returns and merges the pane back via the drain below.
                if ctx.input(|i| i.viewport().close_requested()) {
                    close_requested = true;
                }
                // Root accessibility node for the detached window so a model can find the window by
                // `popout-window-{pane_id}` out-of-process. Emitted via a zero-interaction
                // `egui::Area` (a floating layer) and NOT a `CentralPanel`, because a CentralPanel
                // would consume the viewport's central rect and starve the body's own CentralPanel of
                // space — the round-1 two-panel bug that left the detached window's BODY blank on the
                // real backend. An `Area` reserves no central-panel layout space, so `render_content`
                // below opens the ONLY CentralPanel in this viewport and owns the full central rect.
                let root_id = egui::Id::new(popout_window_author_id(pane_id.as_ref()));
                egui::Area::new(root_id)
                    .fixed_pos(egui::Pos2::ZERO)
                    .interactable(false)
                    .movable(false)
                    .show(ctx, |ui| {
                        // Allocate a real (if tiny) interactive rect so the id is live this frame and
                        // egui attaches the AccessKit node to the tree; the node carries the window
                        // identity, not the body geometry (the body owns its own pane node).
                        ui.interact(ui.max_rect(), root_id, egui::Sense::hover());
                        ctx.accesskit_node_builder(root_id, |node| {
                            node.set_role(accesskit::Role::Window);
                            node.set_author_id(popout_window_author_id(pane_id.as_ref()));
                            node.set_label(title.clone());
                        });
                    });
                // Rich pane content (tab bar + factory body + live pane AccessKit node) via the SAME
                // host render path the main split uses. This opens the SINGLE CentralPanel in the
                // detached viewport, so the body gets the full central rect. Runs on BOTH the embedded
                // (headless) and the native (real-window) path, so the detached window is never blank.
                render_content(ctx, class, pane_id);
            });

            if close_requested {
                if let Some(state) = self.pop_outs.get_mut(pane_id) {
                    state.request_close();
                }
            }
        }

        // ── Drain merged-back entries AFTER show, never during iteration ──────────────────────────
        // Collect the keys to remove first (red-team CONTROL: no double-removal / no mutate-during-
        // iterate). A key appears at most once, so an entry cannot be removed twice in one frame even
        // if both the OS close button and the Merge Back button fired.
        let to_remove: Vec<PaneId> = self
            .pop_outs
            .iter()
            .filter(|(_, s)| !s.open)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &to_remove {
            self.pop_outs.remove(id);
        }
        to_remove
    }
}

/// The detached-window title for a resolved surface label: `"Handshake – <label>"` (en dash). Used by
/// the host, which resolves the label from the registry's `PaneType::label()`.
pub fn popout_title_for(pane_type_label: &str) -> String {
    format!("Handshake \u{2013} {pane_type_label}")
}

/// Stateless renderer for the placeholder tile shown in a popped-out pane's grid rect. Mirrors the
/// `split_layout::SplitLayoutWidget` / `tab_bar::TabBar` stateless-widget convention.
pub struct PopOutPlaceholder;

impl PopOutPlaceholder {
    /// Render the "<label> (popped out)" placeholder + a "Merge Back" button into `ui` for `pane_id`.
    ///
    /// Returns `true` if the Merge Back button was clicked this frame (by pointer OR by an
    /// out-of-process AccessKit `Click`, since the button is a real `egui::Button` whose live node
    /// carries `Action::Click`). The caller wires a `true` return to
    /// [`PopOutManager::merge_back`]. The button gets a stable `author_id`
    /// (`merge-back-{pane_id}`, [`Role::Button`](accesskit::Role::Button)) and, for the four fixed
    /// grid panes, a fixed `NodeId` (64..67) so a model can address it by a stable id.
    ///
    /// `pane_label` is the surface label (`PaneType::label()`), resolved by the caller from the
    /// registry, so the placeholder reads e.g. "Workspace (popped out)".
    pub fn show(
        ui: &mut egui::Ui,
        pane_id: &str,
        pane_label: &str,
        text_color: egui::Color32,
    ) -> bool {
        let mut merge_clicked = false;
        ui.vertical_centered(|ui| {
            ui.add_space(8.0);
            ui.colored_label(text_color, format!("{pane_label} (popped out)"));
            ui.add_space(6.0);

            // Build the Merge Back button with a FIXED egui::Id so its AccessKit NodeId is stable for
            // the four grid panes (64..67). We hand-roll the interactive widget (like the theme
            // toggle in app.rs) because egui::Button does not expose an id override.
            let button_id = merge_back_egui_id(pane_id);
            let label = "Merge Back";
            let galley = ui.painter().layout_no_wrap(
                label.to_owned(),
                egui::FontId::proportional(14.0),
                text_color,
            );
            let padding = ui.spacing().button_padding;
            let desired = galley.size() + padding * 2.0;
            let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
            let response = ui.interact(rect, button_id, egui::Sense::click());
            if ui.is_rect_visible(rect) {
                let visuals = ui.style().interact(&response);
                ui.painter()
                    .rect_filled(rect, visuals.corner_radius, visuals.bg_fill);
                let text_pos = rect.center() - galley.size() * 0.5;
                ui.painter().galley(text_pos, galley, visuals.text_color());
            }
            // Real interactive node: Role::Button + Action::Click via widget_info, then a stable
            // author_id attached to the SAME live node (so it passes the no-unnamed-interactive gate
            // AND is addressable out-of-process).
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
            });
            crate::accessibility::emit_interactive_node(
                ui.ctx(),
                button_id,
                &merge_back_author_id(pane_id),
            );
            if response.clicked() {
                merge_clicked = true;
            }
        });
        merge_clicked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn pid(s: &str) -> PaneId {
        Arc::from(s)
    }

    #[test]
    fn pop_out_inserts_open_state_and_is_popped_out_true() {
        let mut mgr = PopOutManager::new();
        let a = pid("pane-a");
        mgr.pop_out(a.clone(), PopOutGeometry::at(egui::pos2(10.0, 20.0)));
        assert!(mgr.is_popped_out(&a), "pane-a is popped out after pop_out");
        let state = mgr.get(&a).expect("state present");
        assert!(state.open, "fresh pop-out is open");
        assert_eq!(state.geometry.pos, egui::pos2(10.0, 20.0));
        assert_eq!(state.geometry.size, DEFAULT_POPOUT_SIZE);
        assert_eq!(mgr.len(), 1);
    }

    #[test]
    fn merge_back_sets_open_false_then_drain_removes_entry() {
        // Drives the merge-back lifecycle WITHOUT an egui frame by exercising the same state the
        // show_all drain reads: merge_back sets open=false, and a manual drain (mirroring show_all's
        // post-show removal) removes the entry so is_popped_out becomes false.
        let mut mgr = PopOutManager::new();
        let a = pid("pane-a");
        mgr.pop_out(a.clone(), PopOutGeometry::default());
        mgr.merge_back(&a);
        assert!(!mgr.get(&a).unwrap().open, "merge_back marks open=false");
        // is_popped_out is still true until the drain (entry exists, just closed).
        assert!(mgr.is_popped_out(&a), "entry still present pre-drain");

        // Mirror show_all's drain.
        let removed = mgr.drain_closed_for_test();
        assert_eq!(removed, vec![a.clone()], "the closed entry is drained");
        assert!(
            !mgr.is_popped_out(&a),
            "pane-a no longer popped out after drain"
        );
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn double_merge_back_request_does_not_double_remove() {
        // Red-team CONTROL: both the OS close button and the Merge Back button fire in one frame.
        // merge_back + request_close converge on the same open=false flag; the drain removes the
        // entry exactly once (the key appears once in to_remove).
        let mut mgr = PopOutManager::new();
        let a = pid("pane-a");
        mgr.pop_out(a.clone(), PopOutGeometry::default());
        mgr.merge_back(&a); // Merge Back button
        mgr.get_mut_for_test(&a).unwrap().request_close(); // OS close button, same frame
        let removed = mgr.drain_closed_for_test();
        assert_eq!(removed, vec![a.clone()], "removed exactly once, no panic");
        assert_eq!(mgr.len(), 0);
    }

    #[test]
    fn merge_back_unknown_pane_is_noop() {
        let mut mgr = PopOutManager::new();
        mgr.merge_back(&pid("ghost")); // no panic, no entry created
        assert!(mgr.is_empty());
    }

    #[test]
    fn viewport_id_is_stable_for_same_pane_across_pop_outs() {
        // The same pane must reuse the same OS window handle across pop-out/merge-back cycles, so a
        // model/OS that targeted the window keeps its handle.
        let a = pid("pane-a");
        let s1 = PopOutState::new(a.clone(), PopOutGeometry::default());
        let s2 = PopOutState::new(a.clone(), PopOutGeometry::at(egui::pos2(50.0, 50.0)));
        assert_eq!(
            s1.viewport_id, s2.viewport_id,
            "same pane -> same ViewportId across pop-outs"
        );
        // Different panes get different ids.
        let b = PopOutState::new(pid("pane-b"), PopOutGeometry::default());
        assert_ne!(
            s1.viewport_id, b.viewport_id,
            "distinct panes -> distinct viewport ids"
        );
    }

    #[test]
    fn geometry_round_trips_through_serde_json() {
        let g = PopOutGeometry {
            pos: egui::pos2(123.5, 45.25),
            size: egui::vec2(640.0, 480.0),
        };
        let json = serde_json::to_string(&g).expect("serialize");
        let back: PopOutGeometry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, g, "geometry round-trips with no data loss");
    }

    #[test]
    fn off_screen_geometry_clamps_to_fallback() {
        // Red-team CONTROL: a position saved on a now-disconnected monitor lands outside the primary
        // screen rect; clamped_to resets it to the fallback so it can never open off-screen.
        let screen = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1920.0, 1080.0));
        let off = PopOutGeometry {
            pos: egui::pos2(5000.0, 3000.0), // far off the primary monitor
            size: egui::vec2(800.0, 600.0),
        };
        let clamped = off.clamped_to(screen);
        assert_eq!(
            clamped.pos, FALLBACK_POPOUT_POS,
            "off-screen pos reset to fallback"
        );
        assert_eq!(clamped.size, off.size, "size is preserved on clamp");

        // An on-screen position is left untouched.
        let on = PopOutGeometry {
            pos: egui::pos2(100.0, 100.0),
            size: egui::vec2(800.0, 600.0),
        };
        assert_eq!(on.clamped_to(screen), on, "on-screen geometry unchanged");
    }

    #[test]
    fn show_all_does_not_open_a_central_panel_so_body_owns_the_only_one() {
        // REGRESSION (round-2 MAJOR): the detached viewport must contain EXACTLY ONE CentralPanel,
        // owned by the body (`render_content`). The round-1 code opened a SECOND, body-less
        // CentralPanel in `show_all` purely to host the window-root `Role::Window` node. egui treats
        // the CentralPanel as the single "rest of the space" panel keyed by the fixed id
        // `(viewport_id, "central_panel")`; opening it twice produces two overlapping, same-id
        // background panels in one viewport — the body's CentralPanel then fights an identical
        // full-rect sibling instead of cleanly owning the central rect. The fix emits the window-root
        // node from a zero-interaction `egui::Area` (no central-panel allocation), so `show_all`
        // itself opens NO CentralPanel.
        //
        // Headless witness (deterministic, no real winit window needed): drive `show_all` with a
        // NO-OP `render_content`, so the ONLY CentralPanel that could exist is one `show_all` itself
        // opens. After the frame:
        //   * `ctx.read_response((viewport_id, "central_panel"))` is `Some` iff a CentralPanel was
        //     opened this/last pass;
        //   * `ctx.used_rect()` becomes the full content rect iff a CentralPanel was allocated
        //     (`allocate_central_panel` records it; an `Area` does not).
        // Round-1 (CentralPanel) => `Some(..)` + full `used_rect`; this test would FAIL there.
        // Fixed (Area)           => `None` + empty `used_rect`; this test PASSES.
        let ctx = egui::Context::default();
        ctx.enable_accesskit();

        let mut mgr = PopOutManager::new();
        let a = pid("pane-a");
        mgr.pop_out(a.clone(), PopOutGeometry::at(egui::pos2(0.0, 0.0)));

        // Two frames: pass N's widget rects settle from pass N-1 (egui reads prev-pass rects), so the
        // central-panel marker is reliable on the second run.
        for _ in 0..2 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                // NO-OP render_content: the real body's CentralPanel is intentionally NOT opened here,
                // so any CentralPanel that appears was opened by show_all itself (the bug).
                mgr.show_all(
                    ctx,
                    |p| popout_title_for(p.as_ref()),
                    |_ctx, _class, _pane| {},
                );
            });
        }

        let central_panel_id = egui::Id::new((ctx.viewport_id(), "central_panel"));
        assert!(
            ctx.read_response(central_panel_id).is_none(),
            "show_all must NOT open a CentralPanel (the body's render_content owns the sole \
             CentralPanel); a live `central_panel` response means show_all opened a second one"
        );
        assert!(
            !ctx.used_rect().is_positive(),
            "with a no-op render_content show_all must allocate NO central-panel space \
             (used_rect={:?}); a full used_rect means show_all opened a CentralPanel",
            ctx.used_rect()
        );

        // The window-root node is STILL emitted (via the Area) so the detached window stays
        // addressable as popout-window-pane-a — proven by the egui_kittest integration test
        // `popped_out_pane_window_title_and_window_node_are_present`.
    }

    #[test]
    fn show_all_preserves_user_positioned_geometry_across_frames() {
        // FIX 3 regression: a pop-out positioned far outside the primary window's content_rect (e.g.
        // dragged onto a second monitor) must KEEP its position across show_all frames. show_all must
        // NOT clamp/overwrite live geometry against the primary content_rect — that previously snapped
        // such windows back to (100,100) every frame. The off-screen safety net is restore-time only
        // (clamped_to), not a live per-frame clamp.
        let ctx = egui::Context::default();
        ctx.enable_accesskit();

        let mut mgr = PopOutManager::new();
        let a = pid("pane-a");
        // A position deliberately far beyond a headless context's (tiny/zero) content_rect — the same
        // shape as a second-monitor window outside the primary window's content area.
        let far = PopOutGeometry {
            pos: egui::pos2(5000.0, 3000.0),
            size: egui::vec2(640.0, 480.0),
        };
        mgr.pop_out(a.clone(), far);

        // Run several show_all frames; geometry must be untouched after every one.
        for frame in 0..3 {
            let _ = ctx.run(egui::RawInput::default(), |ctx| {
                mgr.show_all(
                    ctx,
                    |p| popout_title_for(p.as_ref()),
                    |_ctx, _class, _pane| {},
                );
            });
            let geo = mgr.get(&a).expect("pop-out still open").geometry;
            assert_eq!(
                geo.pos, far.pos,
                "frame {frame}: user-positioned (second-monitor) pos preserved, not snapped back"
            );
            assert_eq!(geo.size, far.size, "frame {frame}: size preserved");
        }
        assert!(mgr.is_popped_out(&a), "pop-out remains open across frames");
    }

    #[test]
    fn popped_out_ids_are_sorted() {
        let mut mgr = PopOutManager::new();
        mgr.pop_out(pid("pane-c"), PopOutGeometry::default());
        mgr.pop_out(pid("pane-a"), PopOutGeometry::default());
        mgr.pop_out(pid("pane-b"), PopOutGeometry::default());
        let ids: Vec<String> = mgr.popped_out_ids().iter().map(|p| p.to_string()).collect();
        assert_eq!(
            ids,
            vec!["pane-a", "pane-b", "pane-c"],
            "stable sorted order"
        );
    }

    #[test]
    fn title_uses_en_dash_and_resolved_label() {
        assert_eq!(
            popout_title_for("Workspace"),
            "Handshake \u{2013} Workspace"
        );
        assert_eq!(
            popout_title_for("Inference Lab"),
            "Handshake \u{2013} Inference Lab"
        );
    }

    #[test]
    fn merge_back_ids_are_in_fixed_band_and_collision_safe() {
        // The four grid panes map to 64..67, strictly above the tab-bar band (60..63) and below the
        // pane id base (100). Disjoint from chrome (10/20/21) and dividers (30/31).
        let ids: Vec<u64> = MERGE_BACK_SLOTS.iter().map(|(_, id)| *id).collect();
        assert_eq!(ids, vec![64, 65, 66, 67]);
        for id in &ids {
            assert!(*id >= 64 && *id <= 67);
            assert!(*id < crate::accessibility::PANE_NODE_ID_BASE);
        }
        // author_ids follow the documented convention.
        assert_eq!(merge_back_author_id("pane-a"), "merge-back-pane-a");
        assert_eq!(popout_window_author_id("pane-a"), "popout-window-pane-a");
        // Non-grid pane falls back to a hashed (non-fixed) id but a stable author_id.
        assert!(merge_back_node_id("pane-x").is_none());
        assert_eq!(merge_back_author_id("pane-x"), "merge-back-pane-x");
    }

    // ── Test-only helpers mirroring show_all's internal drain so the lifecycle is provable without a
    //    live egui frame (the frame-driven path is proven by the egui_kittest integration test). ────

    impl PopOutManager {
        /// Drain entries with `open == false`, returning removed ids sorted — the exact post-show
        /// removal `show_all` performs. Test-only.
        fn drain_closed_for_test(&mut self) -> Vec<PaneId> {
            let mut to_remove: Vec<PaneId> = self
                .pop_outs
                .iter()
                .filter(|(_, s)| !s.open)
                .map(|(id, _)| id.clone())
                .collect();
            to_remove.sort();
            for id in &to_remove {
                self.pop_outs.remove(id);
            }
            to_remove
        }

        fn get_mut_for_test(&mut self, pane_id: &PaneId) -> Option<&mut PopOutState> {
            self.pop_outs.get_mut(pane_id)
        }
    }
}
