//! Pane-local header binding for the native work surface (WP-KERNEL-011 MT-013).
//!
//! ## What this provides
//!
//! Each pane in the MT-005 [`crate::pane_registry::PaneRegistry`] gets a **header row** rendered
//! above its MT-007 tab strip (which itself sits above the MT-006 pane body). The header binds the
//! pane to its active-tab / module / project context per the C3 contract:
//!
//! - a **title label** bound at render time to the pane's ACTIVE tab label
//!   (`TAB_LABEL_BY_ID[active_tab]` — i.e. [`crate::pane_registry::PaneType::default_label`] of the
//!   active [`crate::tab_bar::TabState`]); the title is read from the live tab-bar state every frame
//!   so a module switch (MT-012) / tab click (MT-007) immediately re-titles the pane (no stale cache);
//! - a right-aligned **lock button** that toggles [`crate::pane_registry::LockState`] on the pane
//!   record (the React `togglePaneLock` / `main-pane__lock`, `app/src/App.tsx` lines 1866-1873);
//! - the **module/type badge** suffix on each tab chip — see [`module_label_for_tab`] — is rendered
//!   by the MT-007 tab bar from the hint this module computes, so a tab reads `Inference Lab (LAB)`.
//!
//! This is the C3 pane-local-binding layer. It is DISTINCT from:
//! - the project (workspace) tabs ([`crate::project_tabs`], MT-011) — project identity, top level;
//! - the top-right module switcher ([`crate::module_switcher`], MT-012) — module context;
//! - the per-pane document tab strip ([`crate::tab_bar`], MT-007) — the tab chips themselves.
//!
//! The header BINDS those layers together onto one pane header: title <- active tab, badge <- module,
//! lock <- pane record.
//!
//! ## Faithful port of the React pane header
//!
//! React renders `.main-pane__header` = `[.main-pane__tabs][.main-pane__lock]`
//! (`app/src/App.tsx` lines 1843-1874). The native shell already renders the tab strip (MT-007) in a
//! strip of its own; MT-013 adds the binding header ABOVE it: the active-tab title (the operator's
//! "what am I looking at" anchor) on the left and the Lock/Unlock control on the right. The lock
//! button's stable id is the React `pane-{id}-lock` `data-stable-id` adopted verbatim as the AccessKit
//! `author_id`.
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! The lock button is the header's one interactive control. The four fixed grid panes (pane-a..d) map
//! to a FRESH fixed `NodeId` band ([`PANE_LOCK_NODE_ID_BASE`] = 70..73), disjoint from every other
//! declared identity (theme toggle 10, chrome 20/21, dividers 30/31, scrollbar rails 40..43,
//! project-tab strip 50, module buttons 51..56, tab-bar containers 60..63, merge-back 64..67, panes at
//! 100 and above). The collision test in [`crate::accessibility::registry`] proves the disjointness across
//! the whole declared set. Each lock button is a `Role::Button` node with `Action::Click`/
//! `Action::Focus` and an `author_id` equal to `pane-{pane_id}-lock` — the same stable key the React
//! `data-stable-id` used — so an out-of-process model toggles a pane's lock by a stable id, not a frame
//! counter. The title label is presentational (a `Role::Label`, no author_id), exactly like the pane
//! body label, so it does NOT trip the MT-025 `assert_no_unnamed_interactive` gate.

use egui::accesskit;

use crate::module_switcher::{ModuleId, MODULE_DEFINITIONS};
use crate::pane_registry::PaneType;

/// Header row height in logical pixels. A compact band (smaller than the
/// [`crate::tab_bar::TAB_BAR_HEIGHT`] 32px strip) carrying the active-tab title + lock control.
pub const PANE_HEADER_HEIGHT: f32 = 24.0;

/// Fixed AccessKit/egui `NodeId` band base for the per-pane LOCK buttons. The four fixed grid panes
/// (pane-a..pane-d) map to 70..73 by their spatial slot — a fresh band placed directly above the
/// MT-008 merge-back band (64..67) and strictly below the pane id base (100), so the collision-free
/// invariant in `accessibility::registry` holds. Declared in
/// [`crate::accessibility::registry::DECLARED_IDENTITIES`] so the collision test covers them.
pub const PANE_LOCK_NODE_ID_BASE: u64 = 70;

/// Fixed AccessKit/egui `NodeId` band base for the per-pane header TITLE labels. The four fixed grid
/// panes map to 74..77, directly above the lock band (70..73) and strictly below the pane id base
/// (100). The title is the pane's bound active-tab identity; emitting it as an addressable
/// `Role::Label` node (with author_id `pane-{pane_id}-title`) lets an out-of-process model READ which
/// tab a pane is showing by a stable id — the core C3 binding the contract asks for — without scraping
/// pixels. A Label is non-interactive, so it does not trip the MT-025 `assert_no_unnamed_interactive`
/// gate even though it carries an author_id.
pub const PANE_TITLE_NODE_ID_BASE: u64 = 74;

/// The four fixed pane slots paired with their lock-button `NodeId`, in the same spatial order as
/// `split_layout`'s 2x2 grid, `tab_bar::TABBAR_SLOTS`, and `popout_window::MERGE_BACK_SLOTS`. A
/// dynamic pane added in a later MT has no fixed slot and falls back to a hashed id (see
/// [`pane_lock_egui_id`]); the four seeded panes are the addressable-by-fixed-id set the acceptance
/// criteria assert by string.
pub const PANE_LOCK_SLOTS: [(&str, u64); 4] = [
    ("pane-a", PANE_LOCK_NODE_ID_BASE),     // 70
    ("pane-b", PANE_LOCK_NODE_ID_BASE + 1), // 71
    ("pane-c", PANE_LOCK_NODE_ID_BASE + 2), // 72
    ("pane-d", PANE_LOCK_NODE_ID_BASE + 3), // 73
];

/// The fixed lock-button `NodeId` for a pane slot, if it is one of the four grid panes.
pub fn pane_lock_node_id(pane_id: &str) -> Option<u64> {
    PANE_LOCK_SLOTS
        .iter()
        .find(|(slot, _)| *slot == pane_id)
        .map(|(_, id)| *id)
}

/// Stable out-of-process author_id for a pane's lock button (`pane-{pane_id}-lock`) — the React
/// `data-stable-id` adopted verbatim (`app/src/App.tsx` line 1871).
pub fn pane_lock_author_id(pane_id: &str) -> String {
    format!("pane-{pane_id}-lock")
}

/// Stable `egui::Id` for a pane's lock button. For the four fixed grid panes this is the fixed-value
/// id (so its AccessKit `NodeId` equals [`pane_lock_node_id`]); for any other pane it is derived from
/// the author_id string so it is still stable across frames. Mirrors
/// [`crate::popout_window::merge_back_egui_id`].
pub fn pane_lock_egui_id(pane_id: &str) -> egui::Id {
    match pane_lock_node_id(pane_id) {
        // # Safety: a single hand-assigned, never-reused fixed id (70..73) cannot self-collide;
        // entropy only affects egui's child IdMap distribution. The band is disjoint from all other
        // declared ids by construction (see PANE_LOCK_NODE_ID_BASE doc).
        Some(node_id) => unsafe { egui::Id::from_high_entropy_bits(node_id) },
        None => egui::Id::new(pane_lock_author_id(pane_id)),
    }
}

/// The four fixed pane slots paired with their header-title `NodeId` (74..77), same spatial order as
/// the other per-pane bands.
pub const PANE_TITLE_SLOTS: [(&str, u64); 4] = [
    ("pane-a", PANE_TITLE_NODE_ID_BASE),     // 74
    ("pane-b", PANE_TITLE_NODE_ID_BASE + 1), // 75
    ("pane-c", PANE_TITLE_NODE_ID_BASE + 2), // 76
    ("pane-d", PANE_TITLE_NODE_ID_BASE + 3), // 77
];

/// The fixed header-title `NodeId` for a pane slot, if it is one of the four grid panes.
pub fn pane_title_node_id(pane_id: &str) -> Option<u64> {
    PANE_TITLE_SLOTS
        .iter()
        .find(|(slot, _)| *slot == pane_id)
        .map(|(_, id)| *id)
}

/// Stable out-of-process author_id for a pane's header title (`pane-{pane_id}-title`). Mirrors the
/// React `data-stable-id` naming convention (`pane-{id}-content`, `pane-{id}-tabs`, etc.).
pub fn pane_title_author_id(pane_id: &str) -> String {
    format!("pane-{pane_id}-title")
}

/// Stable `egui::Id` for a pane's header title. Fixed-value for the four grid panes (so its AccessKit
/// `NodeId` equals [`pane_title_node_id`]); author-id-derived otherwise.
pub fn pane_title_egui_id(pane_id: &str) -> egui::Id {
    match pane_title_node_id(pane_id) {
        // # Safety: a single hand-assigned fixed id (74..77) cannot self-collide; the band is disjoint
        // from all other declared ids by construction (see PANE_TITLE_NODE_ID_BASE doc).
        Some(node_id) => unsafe { egui::Id::from_high_entropy_bits(node_id) },
        None => egui::Id::new(pane_title_author_id(pane_id)),
    }
}

/// The module/type label badge shown as a suffix on a tab chip, derived from the MODULE the tab
/// belongs to (MT-012 [`MODULE_DEFINITIONS`]). This is the contract's `module_label_for_tab`:
///
/// - if the ACTIVE pane module's tab list contains `tab`, return that module's label (so a tab that
///   appears in several modules shows the badge of the pane's current module — the contract's
///   disambiguation rule);
/// - else return the label of the FIRST [`MODULE_DEFINITIONS`] entry whose tab list contains `tab`
///   (its primary/home module);
/// - else return `""` (the tab is in no module — only [`PaneType::Placeholder`], which is never part
///   of any module tab list, hits this).
///
/// Returns a `&'static str` (the module labels are compile-time consts), so the badge is
/// zero-allocation on the hot render path.
pub fn module_label_for_tab(tab: &PaneType, active_module: ModuleId) -> &'static str {
    // 1. Prefer the active pane module if it owns this tab (disambiguation for multi-module tabs).
    let active_def = active_module.definition();
    if active_def.tabs.iter().any(|t| t == tab) {
        return active_def.label;
    }
    // 2. Else the first module whose tab list contains it (its home/primary module).
    for def in MODULE_DEFINITIONS.iter() {
        if def.tabs.iter().any(|t| t == tab) {
            return def.label;
        }
    }
    // 3. In no module: no badge.
    ""
}

/// The interactions a single frame of a pane header produced, surfaced to the caller so the app /
/// split layout can apply them to the pane record. `None` fields mean nothing happened that frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PaneHeaderResponse {
    /// The lock button was clicked this frame (the caller toggles `PaneRecord.lock_state`). The pane
    /// should also become the active pane (mirrors React focusing the pane on a header action).
    pub lock_toggled: bool,
    /// The header (title area or lock) was clicked, so this pane should become the focused pane.
    pub focus_requested: bool,
}

/// Colors the pane header paints with, sourced from the active theme tokens by the caller so the
/// header never reads egui's generic visuals (mirrors [`crate::tab_bar::TabBarColors`]).
#[derive(Debug, Clone, Copy)]
pub struct PaneHeaderColors {
    /// Background fill of the header strip.
    pub bg: egui::Color32,
    /// Active-tab title text color.
    pub title: egui::Color32,
    /// Lock-button label text color.
    pub lock_text: egui::Color32,
    /// Lock-button background fill (idle).
    pub lock_bg: egui::Color32,
    /// Lock-button background fill on hover.
    pub lock_hover_bg: egui::Color32,
    /// Accent used when the pane is LOCKED (so a locked pane reads as locked without pixels).
    pub locked_accent: egui::Color32,
}

/// Stateless renderer for one pane's binding header (title + lock). Borrows its inputs at `show` time
/// and owns nothing (mirrors [`crate::tab_bar::TabBar`] / [`crate::split_layout::SplitLayoutWidget`]).
pub struct PaneHeader;

impl PaneHeader {
    /// Render the pane header into `ui` and return the interactions it produced.
    ///
    /// - `pane_id`: the kebab-case pane key (drives the lock button's stable id).
    /// - `active_tab_label`: the label of the pane's ACTIVE tab, read from the live tab-bar state by
    ///   the caller every frame (so the title binding is never stale). Empty for a pane with no tabs.
    /// - `locked`: whether the pane record is currently locked (drives the Lock/Unlock label + accent).
    /// - `colors`: the MT-003 theme-token colors the header paints with.
    ///
    /// Layout: a horizontal row — the active-tab title on the LEFT (a presentational `Role::Label`),
    /// the Lock/Unlock button right-aligned (`right_to_left`), mirroring the React `.main-pane__header`
    /// flex row. The title is a label (NOT interactive) so it does not need a stable author_id; the
    /// lock button is the one interactive control and carries its fixed id + author_id.
    pub fn show(
        ui: &mut egui::Ui,
        pane_id: &str,
        active_tab_label: &str,
        locked: bool,
        colors: PaneHeaderColors,
    ) -> PaneHeaderResponse {
        let mut response = PaneHeaderResponse::default();

        // Paint the header background across the strip.
        let header_rect = ui.available_rect_before_wrap();
        if ui.is_rect_visible(header_rect) {
            ui.painter().rect_filled(header_rect, 0.0, colors.bg);
        }

        ui.horizontal(|ui| {
            // ── Active-tab title (left) ─────────────────────────────────────────────────────────────
            // Bound to the active tab's label, read live by the caller every frame. Emitted as an
            // ADDRESSABLE `Role::Label` node (fixed id + author_id `pane-{id}-title`) so an
            // out-of-process model reads which tab a pane is showing by a stable id — the core C3
            // binding. A Label is non-interactive, so the MT-025 interactive gate ignores it even with
            // an author_id.
            let title = if active_tab_label.is_empty() {
                "(no tab)"
            } else {
                active_tab_label
            };
            Self::title_label(ui, pane_id, title, colors);

            // ── Lock button (right-aligned) ─────────────────────────────────────────────────────────
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if Self::lock_button(ui, pane_id, locked, colors) {
                    response.lock_toggled = true;
                    response.focus_requested = true;
                }
            });
        });

        response
    }

    /// Render the active-tab title as a real, painted, addressable `Role::Label` node at the pane's
    /// fixed title id. We allocate + interact at the fixed id (rather than `ui.label`, whose id is
    /// auto-generated and whose AccessKit emission egui may coalesce away under clipping) so the title
    /// node is reliably in the live tree with a stable `NodeId` + `author_id` — the same id discipline
    /// the chrome title/status widgets use (`app::title_identity`).
    fn title_label(ui: &mut egui::Ui, pane_id: &str, title: &str, colors: PaneHeaderColors) {
        let id = pane_title_egui_id(pane_id);
        let font = egui::FontId::proportional(13.0);
        let galley = ui.painter().layout_no_wrap(title.to_owned(), font, colors.title);
        // Clamp the title width to the available header width so a long title never pushes the lock
        // button off-strip (the lock is rendered right_to_left after this, so reserving less here keeps
        // it visible). The galley itself is laid out no-wrap; the allocation is what bounds the click box.
        let avail_w = ui.available_width();
        let alloc_w = galley.size().x.min((avail_w - 4.0).max(0.0));
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(alloc_w, galley.size().y),
            egui::Sense::hover(),
        );
        if ui.is_rect_visible(rect) {
            ui.painter().galley(rect.min, galley, colors.title);
        }
        // Register the fixed id in egui's interaction/parent map so the live node attaches under the
        // header scope, then enrich it as a Role::Label with the stable author_id + the title text.
        ui.interact(rect, id, egui::Sense::hover());
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Label);
            node.set_author_id(pane_title_author_id(pane_id));
            node.set_label(title.to_owned());
        });
    }

    /// Render the Lock/Unlock button as a real interactive egui widget at its FIXED AccessKit id and
    /// emit its `Role::Button` node enriched with the stable author_id. Returns `true` if it was
    /// clicked this frame. Mirrors the module-switcher / theme-toggle id discipline: interact at the
    /// fixed id so the Response, its widget_info (Role::Button + label + Action::Click), the AccessKit
    /// bounding box, and the author_id all land on the SAME node.
    fn lock_button(
        ui: &mut egui::Ui,
        pane_id: &str,
        locked: bool,
        colors: PaneHeaderColors,
    ) -> bool {
        let label = if locked { "Unlock" } else { "Lock" };
        let button_id = pane_lock_egui_id(pane_id);

        let text_color = if locked { colors.locked_accent } else { colors.lock_text };
        let font = egui::FontId::proportional(12.0);
        let galley = ui.painter().layout_no_wrap(label.to_owned(), font, text_color);

        let pad_x = 8.0;
        let pad_y = 2.0;
        let desired = egui::vec2(galley.size().x + pad_x * 2.0, galley.size().y + pad_y * 2.0);
        let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
        let response = ui.interact(rect, button_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let bg = if locked {
                colors.locked_accent.gamma_multiply(0.25)
            } else if response.hovered() {
                colors.lock_hover_bg
            } else {
                colors.lock_bg
            };
            ui.painter().rect_filled(rect, 4.0, bg);
            let text_pos = egui::pos2(rect.left() + pad_x, rect.center().y - galley.size().y * 0.5);
            ui.painter().galley(text_pos, galley, text_color);
        }

        // AccessKit: egui derived Action::Click/Action::Focus from Sense::click(); add the Button role
        // + label via widget_info, then attach the stable author_id to the SAME node. `selected`
        // carries the locked state so a model reads lock status without pixels.
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), locked, label)
        });
        ui.ctx().accesskit_node_builder(button_id, |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(pane_lock_author_id(pane_id));
            node.set_label(label.to_owned());
            if locked {
                node.set_selected(true);
            }
        });

        response.clicked()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The 19 non-placeholder PaneType variants (every variant that can appear in a tab strip). Used
    /// by the badge-completeness test, mirroring the contract's "all 19 PaneTabId values".
    fn all_tab_types() -> Vec<PaneType> {
        vec![
            PaneType::Workspace,
            PaneType::LoomDailyJournal,
            PaneType::LoomBlock,
            PaneType::LoomWikiPage,
            PaneType::AtelierEditor,
            PaneType::KernelDcc,
            PaneType::InferenceLab,
            PaneType::ModelRuntime,
            PaneType::Swarm,
            PaneType::Problems,
            PaneType::Jobs,
            PaneType::Timeline,
            PaneType::UserManual,
            PaneType::CodeSymbol,
            PaneType::SourceControl,
            PaneType::MediaDownloader,
            PaneType::FontManager,
            PaneType::FlightRecorder,
            PaneType::VisualDebugger,
        ]
    }

    #[test]
    fn module_label_for_tab_prefers_active_module_then_home() {
        // InferenceLab is in LAB, STAGE, STUDIO. With LAB active it badges LAB.
        assert_eq!(
            module_label_for_tab(&PaneType::InferenceLab, ModuleId::Lab),
            "LAB",
        );
        // Workspace's only module is MAIN; with MAIN active it badges MAIN.
        assert_eq!(
            module_label_for_tab(&PaneType::Workspace, ModuleId::Main),
            "MAIN",
        );
        // Workspace asked for under a module that does NOT contain it (LAB) falls back to its HOME
        // module (the first MODULE_DEFINITIONS entry containing it = MAIN).
        assert_eq!(
            module_label_for_tab(&PaneType::Workspace, ModuleId::Lab),
            "MAIN",
            "tab not in active module falls back to its home module label"
        );
        // AtelierEditor's home module is CKC (it is the CKC default tab, not in MAIN).
        assert_eq!(
            module_label_for_tab(&PaneType::AtelierEditor, ModuleId::Main),
            "CKC",
        );
    }

    #[test]
    fn module_label_for_tab_is_non_empty_for_all_19_tab_types() {
        // The contract's snapshot: every tab type that appears in at least one MODULE_DEFINITIONS
        // entry must badge non-empty for SOME active module. We assert non-empty under the tab's HOME
        // module (Main as the probe; the home-module fallback guarantees a hit regardless).
        for tab in all_tab_types() {
            let badge = module_label_for_tab(&tab, ModuleId::Main);
            assert!(
                !badge.is_empty(),
                "tab {tab:?} must resolve to a non-empty module badge (it is in >=1 module)"
            );
        }
        // Placeholder is in NO module tab list, so it badges empty — the documented no-module case.
        assert_eq!(
            module_label_for_tab(&PaneType::Placeholder("x".to_owned()), ModuleId::Main),
            "",
            "a tab in no module returns an empty badge"
        );
    }

    #[test]
    fn lock_author_id_matches_react_data_stable_id() {
        assert_eq!(pane_lock_author_id("pane-a"), "pane-pane-a-lock");
        // The React id is `pane-${pane.id}-lock`; pane.id is the bare slot ("a","b",..) in React but
        // the native PaneId is the full "pane-a" kebab key, so the native author_id is
        // `pane-pane-a-lock`. This is the native convention (PaneId is the whole key) and is asserted
        // here so a future reader sees the intentional double-`pane` is the native id shape, not a bug.
        assert_eq!(pane_lock_author_id("pane-d"), "pane-pane-d-lock");
    }

    #[test]
    fn lock_node_ids_sit_in_a_disjoint_fresh_band() {
        // The four lock ids are 70..=73: above the merge-back band (64..67), below the pane id base
        // (100), and disjoint from every other fixed id. The full collision proof is in
        // accessibility::registry's collision test.
        let pane_base = crate::accessibility::PANE_NODE_ID_BASE;
        for (slot, id) in PANE_LOCK_SLOTS {
            assert!((70..=73).contains(&id), "lock id {id} for {slot} in the 70..73 band");
            assert!(id < pane_base, "lock id {id} below pane base {pane_base}");
            for fixed in [10_u64, 20, 21, 30, 31, 40, 41, 42, 43, 50, 51, 52, 53, 54, 55, 56, 60, 61, 62, 63, 64, 65, 66, 67] {
                assert_ne!(id, fixed, "lock id {id} collides with fixed id {fixed}");
            }
        }
        // The four ids are distinct.
        let ids: Vec<u64> = PANE_LOCK_SLOTS.iter().map(|(_, id)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "lock ids unique");
    }

    #[test]
    fn title_node_ids_sit_in_a_disjoint_fresh_band() {
        // The four title ids are 74..=77: above the lock band (70..73), below the pane id base (100).
        let pane_base = crate::accessibility::PANE_NODE_ID_BASE;
        for (slot, id) in PANE_TITLE_SLOTS {
            assert!((74..=77).contains(&id), "title id {id} for {slot} in the 74..77 band");
            assert!(id < pane_base, "title id {id} below pane base {pane_base}");
            for fixed in [10_u64, 20, 21, 30, 31, 40, 41, 42, 43, 50, 51, 52, 53, 54, 55, 56, 60, 61, 62, 63, 64, 65, 66, 67, 70, 71, 72, 73] {
                assert_ne!(id, fixed, "title id {id} collides with fixed id {fixed}");
            }
        }
        let ids: Vec<u64> = PANE_TITLE_SLOTS.iter().map(|(_, id)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "title ids unique");
    }

    #[test]
    fn title_author_id_matches_react_naming() {
        assert_eq!(pane_title_author_id("pane-a"), "pane-pane-a-title");
        assert_eq!(pane_title_author_id("pane-d"), "pane-pane-d-title");
    }

    #[test]
    fn lock_egui_id_is_fixed_for_grid_panes_and_stable_for_others() {
        // A grid pane's lock egui id derives from the fixed node id; a non-grid pane derives from the
        // author_id string. Both are stable across calls.
        let a1 = pane_lock_egui_id("pane-a");
        let a2 = pane_lock_egui_id("pane-a");
        assert_eq!(a1, a2, "grid pane lock id stable");
        let x1 = pane_lock_egui_id("pane-x");
        let x2 = pane_lock_egui_id("pane-x");
        assert_eq!(x1, x2, "non-grid pane lock id stable");
        assert_ne!(a1, x1, "distinct panes have distinct lock ids");
    }
}
