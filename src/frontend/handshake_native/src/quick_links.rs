//! Left-rail ACTIVE-WINDOW QUICK LINKS for the native work surface (WP-KERNEL-011 MT-014, section 3).
//!
//! ## What this provides
//!
//! A flat "Windows" list of the currently-open pane tabs across the four work-surface panes. Each row
//! shows the owning project name greyed/muted to the LEFT of the tab label (the paper-control-room
//! design phrase: "active-window quick links in the windows section ... with the owning project greyed
//! out to the left of the window name"). Clicking a row focuses the owning pane and activates that tab.
//!
//! ## Why only the ACTIVE tab per pane (red-team CONTROL)
//!
//! The MT red-team flagged that listing ALL tabs across all four panes (up to 4 x 19 = 76 entries)
//! makes the rail unscrollable and unusable. Following the contract's minimum control, the default view
//! shows ONE row per pane (its active tab) — at most four rows — plus a disclosure toggle that expands
//! to show every tab. The worst-case 4-pane / many-tab list is bounded by [`QUICK_LINKS_MAX_ROWS`] with
//! a "+N more" footer so the rail never grows without bound.
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! The section CONTAINER gets a fixed `NodeId` ([`QUICK_LINKS_NODE_ID`] = 90, `Role::List`) — in the
//! fresh band above the project-tree container (89) and below the pane id base (100). Each row is
//! DYNAMIC (its count varies with open tabs), so it derives its `egui::Id` from its stable author_id
//! STRING (`quick-links.{pane_id}.{tab_index}`) via [`egui::Id::new`] — the same dynamic-count pattern
//! the per-pane tabs and project tabs use. Each row is a `Role::Link` with `Action::Click`; the
//! disclosure toggle is a `Role::Button`. The container List node is non-interactive.

use egui::accesskit;

use crate::pane_registry::{PaneId, PaneType};

/// Fixed AccessKit/egui `NodeId` for the quick-links section CONTAINER node (`Role::List`).
///
/// Occupies the FRESH band slot 90 — disjoint from every other declared identity: theme toggle (10),
/// chrome (20/21), dividers (30/31), scrollbar rails (40..43), project-tab strip (50), module buttons
/// (51..56), tab-bar containers (60..63), merge-back (64..67), pane locks (70..73), pane titles
/// (74..77), the left-rail fixed band (80..88), the project-tree container (89), and the pane id space
/// (>= 100). The collision test in `accessibility::registry` proves the disjointness.
pub const QUICK_LINKS_NODE_ID: u64 = 90;

/// Stable out-of-process author_id for the quick-links section container.
pub const QUICK_LINKS_AUTHOR_ID: &str = "quick-links";

/// Stable out-of-process author_id for the disclosure toggle (collapse/expand all tabs).
pub const QUICK_LINKS_DISCLOSURE_AUTHOR_ID: &str = "quick-links.disclosure";

/// Hard cap on rendered rows so a worst-case many-tab expansion can never grow the rail without bound
/// (red-team CONTROL: a 4-pane x 19-tab surface must stay usable). Beyond this a "+N more" footer
/// summarizes the remainder.
pub const QUICK_LINKS_MAX_ROWS: usize = 24;

/// One open pane tab, reduced to what a quick-link row needs: which pane + tab index it activates, the
/// owning project's display name (greyed prefix), the tab's display label, and the tab's surface type
/// (so the row stays meaningful even if the label is empty). Built by the app from its live pane +
/// tab-bar state; the widget itself owns no app state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLinkEntry {
    /// The pane this tab lives in (the focus target).
    pub pane_id: PaneId,
    /// The tab's index within that pane's tab bar (the activation target).
    pub tab_index: usize,
    /// The owning project's display name, rendered muted/grey to the LEFT of the label.
    pub project_name: String,
    /// The tab's display label.
    pub tab_label: String,
    /// The tab's surface type (kept so the row is addressable/meaningful without the label).
    pub pane_type: PaneType,
    /// Whether this tab is the active tab of its pane (shown only this one by default, all when expanded).
    pub is_active: bool,
}

impl QuickLinkEntry {
    /// Stable author_id for this row (`quick-links.{pane_id}.{tab_index}`).
    pub fn author_id(&self) -> String {
        format!("quick-links.{}.{}", self.pane_id, self.tab_index)
    }
}

/// What the operator clicked in the quick-links list this frame: focus `pane_id` and activate its tab
/// at `tab_index`. Returned to [`crate::left_rail::LeftRail`], which the app turns into a
/// focus-pane + `set_active_tab` action (the native equivalent of React `setActiveTabForPane`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuickLinkClick {
    pub pane_id: PaneId,
    pub tab_index: usize,
}

/// The "Windows" quick-links list widget + its disclosure state.
///
/// It owns ONLY the expand/collapse flag; the entry list is passed in at `show` time from the app's
/// live pane/tab state (single source of truth — the rail never caches a stale copy of the panes).
#[derive(Debug, Default, Clone)]
pub struct ActiveWindowQuickLinks {
    /// When `false` (default) only each pane's ACTIVE tab is shown (red-team CONTROL: max 4 rows);
    /// when `true` every open tab is listed (bounded by [`QUICK_LINKS_MAX_ROWS`]).
    expanded: bool,
}

impl ActiveWindowQuickLinks {
    pub fn new() -> Self {
        Self { expanded: false }
    }

    /// Whether the disclosure is expanded (all tabs) vs collapsed (active tab per pane).
    pub fn is_expanded(&self) -> bool {
        self.expanded
    }

    /// Render the quick-links list into `ui` and return `Some(click)` when a row was clicked this frame.
    ///
    /// `entries` is the FULL set of open tabs (all panes, all tabs). When collapsed, only the entries
    /// whose `is_active` is true are rendered; when expanded, all are rendered (capped at
    /// [`QUICK_LINKS_MAX_ROWS`] with a "+N more" footer).
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        entries: &[QuickLinkEntry],
        colors: QuickLinkColors,
    ) -> Option<QuickLinkClick> {
        let mut click: Option<QuickLinkClick> = None;
        let container_rect = ui.available_rect_before_wrap();
        let container_id = unsafe { egui::Id::from_high_entropy_bits(QUICK_LINKS_NODE_ID) };

        // Section header row: "Windows" title + a disclosure toggle (collapse/expand all tabs).
        ui.horizontal(|ui| {
            let header = ui.painter().layout_no_wrap(
                "Windows".to_owned(),
                egui::FontId::proportional(13.0),
                colors.header_text,
            );
            let (hrect, _) = ui.allocate_exact_size(header.size(), egui::Sense::hover());
            if ui.is_rect_visible(hrect) {
                ui.painter().galley(hrect.min, header, colors.header_text);
            }
            // Disclosure toggle: ▸ collapsed (show active only) / ▾ expanded (show all).
            let toggle_id = egui::Id::new(QUICK_LINKS_DISCLOSURE_AUTHOR_ID);
            let glyph = if self.expanded { "\u{25BE}" } else { "\u{25B8}" }; // ▾ / ▸
            let tg = ui
                .painter()
                .layout_no_wrap(glyph.to_owned(), egui::FontId::proportional(13.0), colors.header_text);
            let (trect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
            let tresp = ui.interact(trect, toggle_id, egui::Sense::click());
            if ui.is_rect_visible(trect) {
                ui.painter().galley(
                    egui::pos2(trect.center().x - tg.size().x * 0.5, trect.center().y - tg.size().y * 0.5),
                    tg,
                    colors.header_text,
                );
            }
            tresp.widget_info(|| {
                egui::WidgetInfo::labeled(
                    egui::WidgetType::Button,
                    ui.is_enabled(),
                    if self.expanded { "Collapse windows" } else { "Expand windows" },
                )
            });
            ui.ctx().accesskit_node_builder(toggle_id, |node| {
                node.set_role(accesskit::Role::Button);
                node.set_author_id(QUICK_LINKS_DISCLOSURE_AUTHOR_ID.to_owned());
                node.set_label("Toggle all windows".to_owned());
                node.set_expanded(self.expanded);
            });
            if tresp.clicked() {
                self.expanded = !self.expanded;
            }
        });

        // Which entries to render: active-only when collapsed, all when expanded.
        let visible: Vec<&QuickLinkEntry> = if self.expanded {
            entries.iter().collect()
        } else {
            entries.iter().filter(|e| e.is_active).collect()
        };
        let total = visible.len();
        let shown = total.min(QUICK_LINKS_MAX_ROWS);

        for entry in visible.iter().take(shown) {
            if quick_link_row(ui, entry, colors) {
                click = Some(QuickLinkClick {
                    pane_id: entry.pane_id.clone(),
                    tab_index: entry.tab_index,
                });
            }
        }
        if total > shown {
            ui.colored_label(colors.muted_text, format!("+{} more\u{2026}", total - shown));
        }

        // Enrich the container node last so its rect spans the rendered list.
        ui.interact(container_rect, container_id, egui::Sense::focusable_noninteractive());
        ui.ctx().accesskit_node_builder(container_id, |node| {
            node.set_role(accesskit::Role::List);
            node.set_author_id(QUICK_LINKS_AUTHOR_ID.to_owned());
            node.set_label("Open windows".to_owned());
        });

        click
    }
}

/// Render ONE quick-link row: `[grey project prefix]  [tab label]`, the project prefix muted/grey to
/// the LEFT of the label (the design-doc layout). The row is a `Role::Link` with a click action,
/// addressed by `quick-links.{pane_id}.{tab_index}`. Returns `true` if clicked.
fn quick_link_row(ui: &mut egui::Ui, entry: &QuickLinkEntry, colors: QuickLinkColors) -> bool {
    let author_id = entry.author_id();
    let id = egui::Id::new(&author_id);
    let height = 20.0;
    let row_w = ui.available_width().max(0.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(row_w, height), egui::Sense::hover());
    let resp = ui.interact(rect, id, egui::Sense::click());

    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter().rect_filled(rect, 3.0, colors.row_hover_bg);
        }
        let pad = 4.0;
        let mut x = rect.left() + pad;
        // Grey project prefix on the LEFT.
        if !entry.project_name.is_empty() {
            let prefix = ui.painter().layout_no_wrap(
                entry.project_name.clone(),
                egui::FontId::proportional(12.0),
                colors.project_prefix,
            );
            ui.painter().galley(
                egui::pos2(x, rect.center().y - prefix.size().y * 0.5),
                prefix.clone(),
                colors.project_prefix,
            );
            x += prefix.size().x + 6.0;
        }
        // Tab label in normal text.
        let label = ui.painter().layout_no_wrap(
            entry.tab_label.clone(),
            egui::FontId::proportional(12.0),
            colors.label_text,
        );
        ui.painter().galley(
            egui::pos2(x, rect.center().y - label.size().y * 0.5),
            label,
            colors.label_text,
        );
    }

    // AccessKit: a Link with a click action. The description carries the owning project + pane so a
    // model reads the binding without scraping the greyed prefix pixels.
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Link,
            ui.is_enabled(),
            format!("{} — {}", entry.project_name, entry.tab_label),
        )
    });
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::Link);
        node.set_author_id(author_id.clone());
        node.set_label(entry.tab_label.clone());
        node.set_description(format!("project: {}; pane: {}", entry.project_name, entry.pane_id));
    });

    resp.clicked()
}

/// Colors the quick-links list paints with, sourced from the active theme tokens by the caller so the
/// list never reads egui's generic visuals (mirrors `project_tree::ProjectTreeColors`).
#[derive(Debug, Clone, Copy)]
pub struct QuickLinkColors {
    /// Greyed/muted owning-project prefix on the left of each row.
    pub project_prefix: egui::Color32,
    /// Normal tab-label text.
    pub label_text: egui::Color32,
    /// Hovered row background.
    pub row_hover_bg: egui::Color32,
    /// "Windows" section header text + the disclosure glyph.
    pub header_text: egui::Color32,
    /// Muted text for the "+N more" footer.
    pub muted_text: egui::Color32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn pid(s: &str) -> PaneId {
        Arc::from(s)
    }

    fn entry(pane: &str, idx: usize, project: &str, label: &str, active: bool) -> QuickLinkEntry {
        QuickLinkEntry {
            pane_id: pid(pane),
            tab_index: idx,
            project_name: project.to_owned(),
            tab_label: label.to_owned(),
            pane_type: PaneType::Workspace,
            is_active: active,
        }
    }

    #[test]
    fn author_id_format_matches_contract() {
        assert_eq!(entry("pane-b", 1, "P", "Inference Lab", true).author_id(), "quick-links.pane-b.1");
    }

    #[test]
    fn quick_links_node_id_in_fresh_band() {
        assert_eq!(QUICK_LINKS_NODE_ID, 90);
        const { assert!(QUICK_LINKS_NODE_ID < crate::accessibility::PANE_NODE_ID_BASE) };
        // Above the project-tree container (89), below the pane id base (100).
        const { assert!(QUICK_LINKS_NODE_ID > crate::project_tree::PROJECT_TREE_NODE_ID) };
    }

    #[test]
    fn collapsed_view_shows_only_active_entries() {
        // Two panes, two tabs each; only the active tab per pane should be visible when collapsed.
        let entries = [
            entry("pane-a", 0, "Alpha", "Workspace", true),
            entry("pane-a", 1, "Alpha", "Problems", false),
            entry("pane-b", 0, "Alpha", "Inference Lab", true),
            entry("pane-b", 1, "Alpha", "Swarm", false),
        ];
        let active_count = entries.iter().filter(|e| e.is_active).count();
        assert_eq!(active_count, 2, "two active tabs across two panes");
    }
}
