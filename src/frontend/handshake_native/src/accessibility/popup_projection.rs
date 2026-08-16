//! Project the LIVE `egui::Context`'s open-popup state into the isolated MCP capture context
//! (WP-KERNEL-012 MT-135).
//!
//! ## The defect this module exists to close
//!
//! `HandshakeApp::refresh_mcp_snapshot` renders the model-vision capture pass on a BRAND-NEW
//! `egui::Context`. A fresh context starts with an empty [`egui::Memory`], and popup/context-menu
//! open state lives in exactly that memory — not in `RawInput`, and not in any app-side field. So a
//! pane context menu that the operator can see on screen was simply absent from the published
//! snapshot: `argus.inspect` reported a window with no menu in it while the window had one open. A
//! model could not see, target, or steer any context menu.
//!
//! MT-121 sized the capture viewport, which fixed COORDINATES only and deliberately left this gap
//! open (`tests/test_mcp_snapshot_viewport.rs`). This module closes it.
//!
//! ## Why ONE mechanism is enough (AC-135-2 — no per-menu allowlist)
//!
//! The pre-MT-135 shell hand-copied exactly two surfaces into the capture context (the per-document
//! Reading-mode store and the tracked open top-menu). Hand-copying a third surface would have
//! recreated the defect the moment a fourth was added.
//!
//! It is not necessary, because of how egui 0.33 actually stores this state. `egui::Memory` keeps
//! **at most one open popup per viewport** — a single `OpenPopup { id, pos }` entry
//! (`egui-0.33.3/src/memory/mod.rs:118`, `:994`). Every memory-backed popup surface in the shell —
//! top menu-bar dropdowns, pane/tab/tree/editor context menus, and any future
//! `egui::Popup::menu` / `egui::Popup::context_menu` / `egui::Popup::open_memory` caller — funnels
//! through that one slot. Therefore "the open popup" is a COMPLETE description of memory-backed
//! popup visibility, and copying that one entry across contexts projects every such surface,
//! present and future, with no registration step and no list of known menus.
//!
//! ## Why this cannot break the MT-121 coordinate contract (AC-135-3)
//!
//! The obvious blunt fix — cloning the whole live `Memory` into the capture context — is rejected
//! here. `Memory` also carries focus, per-widget interaction state, area geometry, and the
//! `data` map that scroll offsets and countless widget states live in. Importing those would change
//! how the capture pass lays out, so published `UiNodeBounds` would stop describing the frame the
//! operator sees, which is precisely the falsehood MT-121 was created to remove.
//!
//! [`OpenPopupProjection`] is therefore a two-field value: the popup's `egui::Id` and its optional
//! stored anchor position. Nothing else crosses the context boundary. The anchor is REQUIRED, not
//! optional polish: `egui::Popup::context_menu` anchors with `PopupAnchor::PointerFixed`, which
//! resolves its rect from the position stored at open time
//! (`egui-0.33.3/src/containers/popup.rs:71`). Re-opening such a popup with no stored position
//! yields `anchor.rect == None` and egui silently renders nothing — so a position-less projection
//! would look like a fix and publish nothing, and a WRONG position would publish a menu somewhere
//! the operator's window does not show it.
//!
//! ## Why a stale or dismissed menu cannot leak (AC-135-5)
//!
//! The projection is READ FRESH from the live context on every capture; it is never cached in app
//! state. egui itself garbage-collects the slot: `Memory::end_pass` drops any popup entry that was
//! not kept open during the pass (`egui-0.33.3/src/memory/mod.rs:773`). So a menu the operator
//! dismissed is gone from live memory before the next capture reads it, and a closed menu projects
//! nothing at all.

/// The complete open-popup state of an `egui::Context`, as a value that can be replayed onto another
/// context.
///
/// Deliberately minimal: this is the entire contract surface between the live context and the
/// isolated capture context. Adding fields here means importing more live state into the capture
/// pass, which is the risk [`open_popup_projection`] exists to bound — see the module docs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpenPopupProjection {
    /// The open popup's `egui::Id`. This is also its `Area`'s layer id
    /// (`egui-0.33.3/src/containers/area.rs:182`), which is what makes discovery below possible.
    pub id: egui::Id,
    /// The anchor position stored when the popup was opened, when it has one. `Some` for
    /// pointer-anchored popups (`PopupAnchor::PointerFixed`, i.e. every right-click context menu and
    /// every keyboard-opened menu routed through `context_menu::request_open`); `None` for popups
    /// anchored to their parent widget, such as the top menu-bar dropdowns.
    pub anchor: Option<egui::Pos2>,
}

/// Read the open memory-backed popup, if any, from `live_ctx`.
///
/// ## How the id is discovered without an allowlist
///
/// egui 0.33 exposes no public "which popup is open" getter — `Memory::popups` is private and
/// `PassState::open_popups` is `pub(crate)`. It does expose two public halves that compose into the
/// answer:
///
/// * `Memory::areas().visible_layer_ids()` — every layer that painted last pass or this pass. An
///   open popup always has one, because `Popup::show` renders its body through `Area::new(popup_id)`
///   and an `Area`'s layer id carries that same `Id`.
/// * `Popup::is_id_open(ctx, id)` — an exact-match probe against the single open-popup slot.
///
/// Probing the visible layer set with the exact-match predicate yields the open popup's id and
/// nothing else. At most one popup can be open, so the result is unique and the unordered layer set
/// cannot make it nondeterministic.
///
/// Returns `None` when no popup is open — the normal, all-menus-closed case.
pub fn open_popup_projection(live_ctx: &egui::Context) -> Option<OpenPopupProjection> {
    // `Memory::everything_is_visible` is egui's debug/benchmark switch that makes EVERY popup id
    // report as open. Under it the exact-match probe below degenerates into "the first visible
    // layer", which would publish an arbitrary surface as an open menu. Refuse to project rather
    // than guess. The shell never enables it; this is a correctness guard, not a live code path.
    if live_ctx.memory(|mem| mem.everything_is_visible()) {
        return None;
    }
    if !egui::Popup::is_any_open(live_ctx) {
        return None;
    }
    let candidates: Vec<egui::Id> = live_ctx.memory(|mem| {
        mem.areas()
            .visible_layer_ids()
            .into_iter()
            .map(|layer| layer.id)
            .collect()
    });
    let id = candidates
        .into_iter()
        .find(|id| egui::Popup::is_id_open(live_ctx, *id))?;
    Some(OpenPopupProjection {
        id,
        anchor: egui::Popup::position_of_id(live_ctx, id),
    })
}

/// Replay `projection` onto `capture_ctx` so the popup renders during the capture pass.
///
/// Writes the id AND the stored anchor position in one memory entry, which is the same primitive
/// egui's own `Popup::show` uses to open a `PointerFixed` popup
/// (`egui-0.33.3/src/containers/popup.rs:515`). `open_popup_at(id, None)` is equivalent to the
/// plain `open_popup(id)` the parent-anchored popups need, so this single call covers both anchor
/// kinds without branching on which surface it is.
///
/// The deprecation on `open_popup_at` points at `PopupAnchor::Position`, which is not applicable
/// here: we are not authoring a popup, we are restoring another context's memory entry for a popup
/// whose own call site chose `PointerFixed`. The position-storing memory open is the matching
/// primitive for that id. `crate::context_menu::request_open` carries the same rationale.
pub fn apply_open_popup_projection(capture_ctx: &egui::Context, projection: OpenPopupProjection) {
    #[allow(deprecated)]
    capture_ctx.memory_mut(|mem| mem.open_popup_at(projection.id, projection.anchor));
}

/// Read the live context's open popup and replay it onto the capture context in one step.
///
/// This is the single documented mechanism `HandshakeApp::refresh_mcp_snapshot` calls. Returns what
/// was projected (`None` when nothing was open) so callers can log or assert on it.
pub fn project_open_popup(
    live_ctx: &egui::Context,
    capture_ctx: &egui::Context,
) -> Option<OpenPopupProjection> {
    let projection = open_popup_projection(live_ctx)?;
    apply_open_popup_projection(capture_ctx, projection);
    Some(projection)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `egui::Context` that has completed a pass with `body` run inside it, so its memory and
    /// area bookkeeping are in the same between-frames state `refresh_mcp_snapshot` reads.
    fn context_after_pass(body: impl FnOnce(&egui::Context)) -> egui::Context {
        let ctx = egui::Context::default();
        // `Context::run` takes an `FnMut` (egui may run several layout passes), so the one-shot body
        // is handed over through an `Option` rather than being moved out of the closure.
        let mut body = Some(body);
        let _ = ctx.run(Default::default(), |ctx| {
            if let Some(body) = body.take() {
                body(ctx);
            }
        });
        ctx
    }

    /// Nothing open projects nothing — the all-menus-closed default (AC-135-5, unit level).
    #[test]
    fn closed_context_projects_nothing() {
        let live = context_after_pass(|_| {});
        assert_eq!(open_popup_projection(&live), None);
    }

    /// A pointer-anchored popup (the context-menu shape) round-trips id AND anchor onto a second,
    /// independent context. Without the anchor a `PointerFixed` popup cannot resolve its rect and
    /// renders nothing, so the anchor is part of the correctness contract, not a nicety.
    #[test]
    fn pointer_anchored_popup_round_trips_id_and_anchor() {
        let popup_id = egui::Id::new("mt135-round-trip").with("popup");
        let anchor = egui::pos2(321.0, 123.0);
        let live = context_after_pass(|ctx| {
            #[allow(deprecated)]
            ctx.memory_mut(|mem| mem.open_popup_at(popup_id, Some(anchor)));
            // A real popup paints an Area under its own id; discovery reads the visible layer set,
            // so the witness has to register the same layer the real popup would.
            egui::Area::new(popup_id).show(ctx, |ui| {
                ui.label("menu body");
            });
        });

        let projected = open_popup_projection(&live).expect("the open popup is discoverable");
        assert_eq!(projected.id, popup_id);
        assert_eq!(projected.anchor, Some(anchor));

        let capture = egui::Context::default();
        apply_open_popup_projection(&capture, projected);
        let mut open_in_capture = false;
        let mut anchor_in_capture = None;
        let _ = capture.run(Default::default(), |ctx| {
            open_in_capture = egui::Popup::is_id_open(ctx, popup_id);
            anchor_in_capture = egui::Popup::position_of_id(ctx, popup_id);
        });
        assert!(
            open_in_capture,
            "the capture context reports the projected popup as open"
        );
        assert_eq!(
            anchor_in_capture,
            Some(anchor),
            "a PointerFixed popup needs its stored anchor to resolve a rect; without it egui \
silently renders nothing"
        );
    }

    /// A parent-anchored popup (the top menu-bar shape) has no stored position, and the projection
    /// carries that faithfully instead of inventing one.
    #[test]
    fn parent_anchored_popup_projects_without_an_anchor() {
        let popup_id = egui::Id::new("mt135-no-anchor").with("popup");
        let live = context_after_pass(|ctx| {
            egui::Popup::open_id(ctx, popup_id);
            egui::Area::new(popup_id).show(ctx, |ui| {
                ui.label("menu body");
            });
        });

        let projected = open_popup_projection(&live).expect("the open popup is discoverable");
        assert_eq!(projected.id, popup_id);
        assert_eq!(projected.anchor, None);
    }

    /// Dismissing the popup in the live context stops the projection immediately — the mechanism
    /// reads live state per capture and never latches (AC-135-5, unit level).
    #[test]
    fn dismissed_popup_stops_projecting() {
        let popup_id = egui::Id::new("mt135-dismiss").with("popup");
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |ctx| {
            egui::Popup::open_id(ctx, popup_id);
            egui::Area::new(popup_id).show(ctx, |ui| {
                ui.label("menu body");
            });
        });
        assert!(open_popup_projection(&ctx).is_some(), "precondition: open");

        let _ = ctx.run(Default::default(), |ctx| {
            egui::Popup::close_id(ctx, popup_id);
        });
        assert_eq!(
            open_popup_projection(&ctx),
            None,
            "a dismissed popup must not keep projecting into the capture pass"
        );
    }
}
