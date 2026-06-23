//! The LEFT ACTIVITY RAIL for the native work surface (WP-KERNEL-011 MT-014).
//!
//! ## What this provides
//!
//! The narrow collapsible sidebar on the far left of the shell, rendered as an
//! `egui::SidePanel::left("left-rail")`. It consolidates what the React UI split across the left-rail
//! module buttons (moved to the top-right module switcher in MT-012), the project-tabs section (moved
//! to the top strip in MT-011), and the `WorkspaceSidebar` file drawer. The rail stacks five sections
//! top-to-bottom (per the MT-014 contract + the paper-control-room design doc):
//!
//! 1. ACTIVITY ICONS — an always-visible 32px icon strip: Files / Agenda / Mail / Notes. Clicking an
//!    icon toggles that section's expansion in the panel below (VS Code activity-bar model).
//! 2. PROJECT TREE — the active project's documents + canvases as a paper-strip tree
//!    ([`crate::project_tree::ProjectTree`]). Clicking a row opens it in the active pane.
//! 3. ACTIVE-WINDOW QUICK LINKS — the "Windows" list of open pane tabs
//!    ([`crate::quick_links::ActiveWindowQuickLinks`]). Clicking a row focuses that pane + tab.
//! 4. STASH AFFORDANCE — a single toggle that opens/closes the bottom drawer stash (the full stash UI
//!    is MT-022; this MT is the entry point only).
//! 5. AGENDA / MAIL / NOTES AFFORDANCES — three bottom buttons that open the corresponding pane tab on
//!    the active pane (Notes -> [`PaneType::LoomDailyJournal`], which exists; Agenda/Mail are future
//!    surfaces and open a labeled placeholder tab for now).
//!
//! ## Collapsing
//!
//! When the rail is COLLAPSED only the activity icon strip + the collapse toggle are visible (32px
//! wide). When OPEN the full panel (default 220px, resizable) shows sections 2-5. The open/closed flag
//! is owned by the app and persisted in the layout snapshot's `drawers.project` field (mirroring the
//! React `projectDrawerOpen` -> `drawers.project` persistence) so it round-trips across sessions.
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! Every interactive control gets a FIXED `egui::Id` in the dedicated 80..=88 fresh band, disjoint from
//! every other declared identity (see [`crate::accessibility::registry`]):
//!
//! - `left-rail.activity.files`   -> 80   (Role::Button)
//! - `left-rail.activity.agenda`  -> 81   (Role::Button)
//! - `left-rail.activity.mail`    -> 82   (Role::Button)
//! - `left-rail.activity.notes`   -> 83   (Role::Button)
//! - `left-rail.stash-toggle`     -> 84   (Role::Button)
//! - `left-rail.agenda`           -> 85   (Role::Button)
//! - `left-rail.mail`             -> 86   (Role::Button)
//! - `left-rail.notes`            -> 87   (Role::Button)
//! - `left-rail.collapse-toggle`  -> 88   (Role::Button)
//!
//! The project-tree container (89) and quick-links container (90) live in their own modules. A
//! fixed-value `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames + restarts —
//! the same convention the theme toggle / chrome / module buttons use.

use egui::accesskit;

use crate::pane_registry::PaneType;
use crate::project_tree::{ProjectTree, ProjectTreeColors, ProjectTreeEvent};
use crate::quick_links::{ActiveWindowQuickLinks, QuickLinkColors, QuickLinkEntry};

/// Base `NodeId` for the left-rail fixed id band (80..=88). Each control's id is `BASE + its slot`.
pub const LEFT_RAIL_NODE_ID_BASE: u64 = 80;

/// The nine fixed left-rail controls, in (author_id, slot-offset) order. The collision test in
/// `accessibility::registry` enumerates these via [`LEFT_RAIL_BUTTONS`] so a new control added here is
/// automatically covered. Slots: activity files/agenda/mail/notes (0..3), stash toggle (4), bottom
/// agenda/mail/notes affordances (5..7), collapse toggle (8).
pub const LEFT_RAIL_BUTTONS: [(&str, u64); 9] = [
    ("left-rail.activity.files", LEFT_RAIL_NODE_ID_BASE),      // 80
    ("left-rail.activity.agenda", LEFT_RAIL_NODE_ID_BASE + 1), // 81
    ("left-rail.activity.mail", LEFT_RAIL_NODE_ID_BASE + 2),   // 82
    ("left-rail.activity.notes", LEFT_RAIL_NODE_ID_BASE + 3),  // 83
    ("left-rail.stash-toggle", LEFT_RAIL_NODE_ID_BASE + 4),    // 84
    ("left-rail.agenda", LEFT_RAIL_NODE_ID_BASE + 5),          // 85
    ("left-rail.mail", LEFT_RAIL_NODE_ID_BASE + 6),            // 86
    ("left-rail.notes", LEFT_RAIL_NODE_ID_BASE + 7),           // 87
    ("left-rail.collapse-toggle", LEFT_RAIL_NODE_ID_BASE + 8), // 88
];

// Stable author_id constants (the canonical match keys an out-of-process model addresses).
pub const ACTIVITY_FILES_AUTHOR_ID: &str = "left-rail.activity.files";
pub const ACTIVITY_AGENDA_AUTHOR_ID: &str = "left-rail.activity.agenda";
pub const ACTIVITY_MAIL_AUTHOR_ID: &str = "left-rail.activity.mail";
pub const ACTIVITY_NOTES_AUTHOR_ID: &str = "left-rail.activity.notes";
pub const STASH_TOGGLE_AUTHOR_ID: &str = "left-rail.stash-toggle";
pub const AGENDA_AUTHOR_ID: &str = "left-rail.agenda";
pub const MAIL_AUTHOR_ID: &str = "left-rail.mail";
pub const NOTES_AUTHOR_ID: &str = "left-rail.notes";
pub const COLLAPSE_TOGGLE_AUTHOR_ID: &str = "left-rail.collapse-toggle";

/// The five activity sections an icon button toggles. Files drives the project-tree visibility; the
/// other three are bottom affordances whose open state is reserved for future per-section panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivitySection {
    Files,
    Agenda,
    Mail,
    Notes,
}

/// The expand/collapse state of the rail's activity sections (which icon's panel is showing). Defaults
/// to Files open so a fresh rail shows the project tree (the most-used surface).
#[derive(Debug, Clone)]
pub struct LeftRailState {
    pub files_open: bool,
    pub agenda_open: bool,
    pub mail_open: bool,
    pub notes_open: bool,
}

impl Default for LeftRailState {
    fn default() -> Self {
        Self {
            files_open: true,
            agenda_open: false,
            mail_open: false,
            notes_open: false,
        }
    }
}

impl LeftRailState {
    /// Toggle the section a clicked activity icon owns, returning the section that changed.
    pub fn toggle(&mut self, section: ActivitySection) {
        match section {
            ActivitySection::Files => self.files_open = !self.files_open,
            ActivitySection::Agenda => self.agenda_open = !self.agenda_open,
            ActivitySection::Mail => self.mail_open = !self.mail_open,
            ActivitySection::Notes => self.notes_open = !self.notes_open,
        }
    }

    /// Whether the given section is currently open.
    pub fn is_open(&self, section: ActivitySection) -> bool {
        match section {
            ActivitySection::Files => self.files_open,
            ActivitySection::Agenda => self.agenda_open,
            ActivitySection::Mail => self.mail_open,
            ActivitySection::Notes => self.notes_open,
        }
    }
}

/// What the operator triggered in the rail this frame, surfaced to [`crate::app::HandshakeApp`] which
/// applies it against the live registry / tab state (single source of truth; the rail owns no pane
/// state). All variants are mutually exclusive per frame (one click resolves to one event).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeftRailEvent {
    /// A project-tree row was clicked: open this document in the active pane.
    OpenDocument(String),
    /// A project-tree row was clicked: open this canvas in the active pane.
    OpenCanvas(String),
    /// A Bookmarks-group row was clicked (MT-014 FIX-A): open the pinned content on the active pane.
    /// When `document_id` is `Some`, open it as that document; otherwise open the pinned Loom block by
    /// `block_id` (matching React `handleOpenBookmark`).
    OpenBookmark {
        document_id: Option<String>,
        block_id: String,
    },
    /// MT-020 explorer-row context menu: "Copy Path" — copy this stable id/path to the clipboard.
    CopyPath(String),
    /// MT-020 explorer-row context menu: "Rename" — rename this Loom block via the verified backend
    /// PATCH endpoint. Carries the block id + current title (the rename-field seed).
    RenameBlock {
        block_id: String,
        current_title: String,
    },
    /// MT-033 explorer-row context menu: "Route to Stage" — route this DOCUMENT to the Stage pane via
    /// the MT-031 Route-to-Stage command. Carries the document id + title the Stage pane displays.
    RouteToStage {
        document_id: String,
        title: String,
    },
    /// The Retry button on a failed project-tree load was clicked.
    RetryProjectTree,
    /// A quick-link row was clicked: focus this pane and activate its tab at `tab_index`.
    FocusPaneTab { pane_id: crate::pane_registry::PaneId, tab_index: usize },
    /// The stash toggle was clicked: flip the bottom drawer open/closed.
    ToggleStash,
    /// A bottom affordance was clicked: open the corresponding tab on the active pane.
    OpenModuleTab(PaneType),
    /// The rail collapse toggle was clicked: flip the rail open/closed (persisted as drawers.project).
    ToggleRail,
}

/// The left activity rail widget. Owns the per-section expand state ([`LeftRailState`]), the project
/// tree ([`ProjectTree`]), and the quick-links list ([`ActiveWindowQuickLinks`]). It does NOT own the
/// rail's OPEN/closed flag — that is app state persisted in the layout snapshot — nor pane state.
pub struct LeftRail {
    pub state: LeftRailState,
    pub project_tree: ProjectTree,
    pub quick_links: ActiveWindowQuickLinks,
}

impl Default for LeftRail {
    fn default() -> Self {
        Self::new()
    }
}

impl LeftRail {
    pub fn new() -> Self {
        Self {
            state: LeftRailState::default(),
            project_tree: ProjectTree::new(),
            quick_links: ActiveWindowQuickLinks::new(),
        }
    }

    /// Render the rail into `ui` and return the (at most one) event the operator triggered this frame.
    ///
    /// `rail_open` is the app-owned open/closed flag (persisted as `drawers.project`). When `false`,
    /// only the activity icon strip + the collapse toggle render; when `true`, sections 2-5 also render.
    ///
    /// `quick_link_entries` is the app's live set of open pane tabs (single source of truth); the rail
    /// reads it each frame rather than caching a copy.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        rail_open: bool,
        quick_link_entries: &[QuickLinkEntry],
        colors: LeftRailColors,
    ) -> Option<LeftRailEvent> {
        let mut event: Option<LeftRailEvent> = None;

        // ── Section 1: ACTIVITY ICON STRIP (always visible) + the collapse toggle ────────────────
        // A narrow vertical column of icon buttons. Clicking an activity icon toggles its section; the
        // collapse toggle flips the whole rail open/closed.
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(32.0);
                // Collapse toggle at the very top so it is reachable in both states.
                let collapse_glyph = if rail_open { "\u{25C2}" } else { "\u{25B8}" }; // ◂ / ▸
                if Self::icon_button(ui, collapse_glyph, COLLAPSE_TOGGLE_AUTHOR_ID, "Collapse rail", false, colors) {
                    event = Some(LeftRailEvent::ToggleRail);
                }
                ui.add_space(4.0);
                for (section, glyph, author_id, tip) in [
                    (ActivitySection::Files, "\u{1F4C1}", ACTIVITY_FILES_AUTHOR_ID, "Files"),
                    (ActivitySection::Agenda, "\u{1F4C5}", ACTIVITY_AGENDA_AUTHOR_ID, "Agenda"),
                    (ActivitySection::Mail, "\u{2709}", ACTIVITY_MAIL_AUTHOR_ID, "Mail"),
                    (ActivitySection::Notes, "\u{1F4DD}", ACTIVITY_NOTES_AUTHOR_ID, "Notes"),
                ] {
                    let active = self.state.is_open(section);
                    if Self::icon_button(ui, glyph, author_id, tip, active, colors) && event.is_none() {
                        self.state.toggle(section);
                    }
                }
            });

            // ── Sections 2-5 render only when the rail is OPEN ───────────────────────────────────
            if rail_open {
                ui.separator();
                ui.vertical(|ui| {
                    // NOTE (no egui ScrollArea): an egui `ScrollArea` registers an internal focusable
                    // drag-to-scroll viewport node (Role::Unknown, click+focus, no author_id) whose live
                    // id egui derives internally and does not surface in a stable way. That anonymous
                    // interactive node trips the MT-025 `assert_no_unnamed_interactive` gate, which is a
                    // hard preservation requirement. Rather than guess egui's internal scroll-node id
                    // (fragile, version-coupled), the rail renders its content directly inside the
                    // SidePanel, which clips overflow. The rail's own red-team scroll controls are still
                    // honored WITHOUT a scroll area: the quick-links list shows only each pane's ACTIVE
                    // tab by default (max 4 rows) with a bounded "+N more" footer, and the project tree
                    // caches its paper-strip label widths so a large project does not pay a per-frame
                    // cost. A future MT can add a gate-clean custom scroll viewport if a very tall tree
                    // needs it. (Recorded as a deviation in the MT-014 handoff.)

                    // Section 2: PROJECT TREE (only while the Files activity is open).
                    if self.state.files_open {
                        let tree_colors = colors.tree_colors();
                        if let Some(tree_event) = self.project_tree.show(ui, tree_colors) {
                            event = Some(match tree_event {
                                ProjectTreeEvent::OpenDocument(id) => LeftRailEvent::OpenDocument(id),
                                ProjectTreeEvent::OpenCanvas(id) => LeftRailEvent::OpenCanvas(id),
                                ProjectTreeEvent::OpenBookmark { document_id, block_id } => {
                                    LeftRailEvent::OpenBookmark { document_id, block_id }
                                }
                                ProjectTreeEvent::CopyPath(id) => LeftRailEvent::CopyPath(id),
                                ProjectTreeEvent::RouteToStage { document_id, title } => {
                                    LeftRailEvent::RouteToStage { document_id, title }
                                }
                                ProjectTreeEvent::RenameBlock { block_id, current_title } => {
                                    LeftRailEvent::RenameBlock { block_id, current_title }
                                }
                            });
                        }
                        if self.project_tree.take_retry_request() && event.is_none() {
                            event = Some(LeftRailEvent::RetryProjectTree);
                        }
                        ui.add_space(8.0);
                    }

                    // Section 3: ACTIVE-WINDOW QUICK LINKS ("Windows").
                    let ql_colors = colors.quick_link_colors();
                    if let Some(click) = self.quick_links.show(ui, quick_link_entries, ql_colors) {
                        if event.is_none() {
                            event = Some(LeftRailEvent::FocusPaneTab {
                                pane_id: click.pane_id,
                                tab_index: click.tab_index,
                            });
                        }
                    }
                    ui.add_space(8.0);
                    ui.separator();

                    // Section 4: STASH affordance (entry point only; full UI is MT-022).
                    if Self::text_button(ui, "\u{1F4E5} Stash", STASH_TOGGLE_AUTHOR_ID, colors) && event.is_none() {
                        event = Some(LeftRailEvent::ToggleStash);
                    }

                    // Section 5: AGENDA / MAIL / NOTES bottom affordances.
                    // Notes -> LoomDailyJournal (a real tab). Agenda/Mail are future surfaces; they open
                    // a labeled Placeholder tab so the affordance is wired end-to-end without inventing a
                    // pane type that does not exist yet.
                    // TODO(MT-future): replace the Agenda/Mail placeholders with PaneType::Agenda /
                    // PaneType::Mail once those surfaces land.
                    if Self::text_button(ui, "\u{1F4C5} Agenda", AGENDA_AUTHOR_ID, colors) && event.is_none() {
                        event = Some(LeftRailEvent::OpenModuleTab(PaneType::Placeholder("Agenda".to_owned())));
                    }
                    if Self::text_button(ui, "\u{2709} Mail", MAIL_AUTHOR_ID, colors) && event.is_none() {
                        event = Some(LeftRailEvent::OpenModuleTab(PaneType::Placeholder("Mail".to_owned())));
                    }
                    if Self::text_button(ui, "\u{1F4DD} Notes", NOTES_AUTHOR_ID, colors) && event.is_none() {
                        event = Some(LeftRailEvent::OpenModuleTab(PaneType::LoomDailyJournal));
                    }
                });
            }
        });

        event
    }

    /// Render one square activity/collapse icon button at its FIXED id and emit a `Role::Button` node
    /// (so it is addressable out-of-process and passes the MT-025 interactive-naming gate). `active`
    /// highlights the button (the section it owns is open). Returns `true` if clicked this frame.
    fn icon_button(
        ui: &mut egui::Ui,
        glyph: &str,
        author_id: &str,
        tooltip: &str,
        active: bool,
        colors: LeftRailColors,
    ) -> bool {
        let node_id = button_node_id(author_id);
        let id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
        let size = egui::vec2(28.0, 28.0);
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let resp = ui
            .interact(rect, id, egui::Sense::click())
            .on_hover_text(tooltip);
        if ui.is_rect_visible(rect) {
            let bg = if active {
                colors.icon_active_bg
            } else if resp.hovered() {
                colors.icon_hover_bg
            } else {
                colors.icon_bg
            };
            ui.painter().rect_filled(rect, 4.0, bg);
            let g = ui
                .painter()
                .layout_no_wrap(glyph.to_owned(), egui::FontId::proportional(15.0), colors.icon_text);
            ui.painter().galley(
                egui::pos2(rect.center().x - g.size().x * 0.5, rect.center().y - g.size().y * 0.5),
                g,
                colors.icon_text,
            );
        }
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), tooltip));
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(author_id.to_owned());
            node.set_label(tooltip.to_owned());
            if active {
                node.set_selected(true);
            }
        });
        resp.clicked()
    }

    /// Render one full-width text button (Stash / Agenda / Mail / Notes affordances) at its FIXED id and
    /// emit a `Role::Button` node. Returns `true` if clicked this frame.
    fn text_button(ui: &mut egui::Ui, label: &str, author_id: &str, colors: LeftRailColors) -> bool {
        let node_id = button_node_id(author_id);
        let id = unsafe { egui::Id::from_high_entropy_bits(node_id) };
        let height = 22.0;
        let width = ui.available_width().max(0.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        let resp = ui.interact(rect, id, egui::Sense::click());
        if ui.is_rect_visible(rect) {
            let bg = if resp.hovered() { colors.icon_hover_bg } else { colors.icon_bg };
            ui.painter().rect_filled(rect, 3.0, bg);
            let g = ui
                .painter()
                .layout_no_wrap(label.to_owned(), egui::FontId::proportional(13.0), colors.icon_text);
            ui.painter().galley(
                egui::pos2(rect.left() + 6.0, rect.center().y - g.size().y * 0.5),
                g,
                colors.icon_text,
            );
        }
        resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Button);
            node.set_author_id(author_id.to_owned());
            node.set_label(label.to_owned());
        });
        resp.clicked()
    }
}

/// The fixed `NodeId` for a left-rail control by its author_id. Returns the band slot from
/// [`LEFT_RAIL_BUTTONS`]; panics in debug if an unknown author_id is passed (a programming error —
/// every rail control must be declared in the band table).
fn button_node_id(author_id: &str) -> u64 {
    LEFT_RAIL_BUTTONS
        .iter()
        .find(|(a, _)| *a == author_id)
        .map(|(_, id)| *id)
        .unwrap_or_else(|| panic!("left-rail control '{author_id}' not declared in LEFT_RAIL_BUTTONS"))
}

/// Colors the rail paints with, sourced from the active theme tokens by the caller so the rail never
/// reads egui's generic visuals (mirrors `project_tabs::ProjectTabColors`). Carries the child-widget
/// color bundles too so the project tree + quick links flip dark<->light with the shell.
#[derive(Debug, Clone, Copy)]
pub struct LeftRailColors {
    pub icon_bg: egui::Color32,
    pub icon_hover_bg: egui::Color32,
    pub icon_active_bg: egui::Color32,
    pub icon_text: egui::Color32,
    pub row_bg: egui::Color32,
    pub row_hover_bg: egui::Color32,
    pub row_text: egui::Color32,
    pub group_text: egui::Color32,
    pub muted_text: egui::Color32,
    pub error: egui::Color32,
    pub project_prefix: egui::Color32,
}

impl LeftRailColors {
    fn tree_colors(&self) -> ProjectTreeColors {
        ProjectTreeColors {
            row_bg: self.row_bg,
            row_hover_bg: self.row_hover_bg,
            row_text: self.row_text,
            group_text: self.group_text,
            muted_text: self.muted_text,
            error: self.error,
        }
    }

    fn quick_link_colors(&self) -> QuickLinkColors {
        QuickLinkColors {
            project_prefix: self.project_prefix,
            label_text: self.row_text,
            row_hover_bg: self.row_hover_bg,
            header_text: self.group_text,
            muted_text: self.muted_text,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn left_rail_band_is_disjoint_and_sequential() {
        // The nine controls occupy 80..=88, strictly below the pane id base (100) and the project-tree
        // (89) / quick-links (90) containers. The full collision proof is in accessibility::registry.
        for (i, (_, id)) in LEFT_RAIL_BUTTONS.iter().enumerate() {
            assert_eq!(*id, LEFT_RAIL_NODE_ID_BASE + i as u64, "sequential band slot");
            assert!(*id < crate::accessibility::PANE_NODE_ID_BASE);
            assert!(*id < crate::project_tree::PROJECT_TREE_NODE_ID, "below project-tree (89)");
        }
        // Ids are unique.
        let ids: Vec<u64> = LEFT_RAIL_BUTTONS.iter().map(|(_, id)| *id).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "left-rail ids unique");
    }

    #[test]
    fn toggle_flips_section_state() {
        let mut state = LeftRailState::default();
        assert!(state.files_open, "files open by default");
        state.toggle(ActivitySection::Files);
        assert!(!state.files_open, "files toggled closed");
        assert!(!state.agenda_open, "agenda closed by default");
        state.toggle(ActivitySection::Agenda);
        assert!(state.agenda_open, "agenda toggled open");
    }

    #[test]
    fn button_node_id_resolves_every_declared_control() {
        for (author_id, expected) in LEFT_RAIL_BUTTONS {
            assert_eq!(button_node_id(author_id), expected);
        }
    }
}
