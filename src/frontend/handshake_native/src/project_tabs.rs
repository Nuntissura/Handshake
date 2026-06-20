//! Top project (workspace) tabs for the native work surface (WP-KERNEL-011 MT-011).
//!
//! ## What this provides
//!
//! A horizontal strip of browser-style tabs, ONE per open Handshake workspace (project), rendered at
//! the top of the shell ABOVE the 2x2 pane grid (between the title bar and the central panel). This
//! is the C3 navigation layer for switching between whole projects — DISTINCT from the per-pane tab
//! bar (MT-007 [`crate::tab_bar`]), which switches documents inside a single pane.
//!
//! Clicking a project tab that is not the active one drives [`crate::app::HandshakeApp`]'s
//! `active_project_id`, which the MT-009 persistence lifecycle keys on: switching a project SAVES the
//! current pane layout for the project being left and LOADS the stored layout for the project being
//! entered (or the default layout if none is stored). The widget itself owns no layout state — it only
//! reports "the operator clicked a different project"; the app performs the save/load (single source
//! of truth for layout, mirroring the React `selectProject()` in `app/src/App.tsx`).
//!
//! ## State model
//!
//! - [`ProjectItem`] — one open project (a stable `id` + display `name`), mirroring the React
//!   `ProjectItem` type and mapped from a backend `Workspace` row.
//! - [`ProjectTabBar`] — the ordered list of [`ProjectItem`]s plus the active project id and the
//!   loading/error state of the last workspace fetch.
//!
//! ## Stable AccessKit ids (out-of-process steering)
//!
//! The tab-strip CONTAINER gets a fixed `NodeId` in a dedicated fresh band ([`PROJECT_TABS_NODE_ID`]
//! = 50) so an out-of-process model can find the strip by the stable author_id `project-tabs`
//! (`Role::TabList`). Individual project tabs are DYNAMIC (their count varies as projects open/close),
//! so each tab's `egui::Id` is derived from its stable author_id STRING `project-tab-{id}` via
//! `egui::Id::new` — stable for a given project id across frames, in egui's hashed id space, clear of
//! the small fixed band used by chrome/dividers/tab-bars/panes. Each tab is a `Role::Tab` node with
//! `Action::Click`/`Action::Focus`; the active tab is marked `selected`. This matches the convention
//! the MT-007 per-tab nodes use (dynamic count -> hashed string id) and the fresh-band convention the
//! MT-010 scrollbar rails use (40..43) and merge-back buttons use (64..67).

use egui::accesskit;

use crate::error::AppError;

/// Fixed AccessKit/egui `NodeId` for the project-tab-strip CONTAINER (`Role::TabList`).
///
/// Occupies the FRESH band slot 50 — disjoint from every other declared identity: theme toggle (10),
/// chrome (20/21), dividers (30/31), scrollbar rails (40..43), tab-bar containers (60..63),
/// merge-back buttons (64..67), and the pane id space (>= 100). The collision test in
/// `accessibility::registry` proves the disjointness across the whole declared set. A fixed-value
/// `egui::Id` (`from_high_entropy_bits`) yields a fixed `NodeId` across frames + process restarts —
/// the same convention the divider/chrome/toggle use — so a model addresses the strip by a stable id.
pub const PROJECT_TABS_NODE_ID: u64 = 50;

/// Stable out-of-process author_id for the project-tab-strip container.
pub const PROJECT_TABS_AUTHOR_ID: &str = "project-tabs";

/// Author_id for the single disabled placeholder tab shown when no projects exist.
pub const NO_PROJECTS_AUTHOR_ID: &str = "project-tab-none";

/// Height of the slim project-tab strip in logical pixels. Slightly slimmer than the per-pane tab bar
/// (32px, MT-007) so the two tab strips read as different navigation layers (paper-strip aesthetic).
pub const PROJECT_TAB_BAR_HEIGHT: f32 = 26.0;

/// Maximum tab label width before truncation with an ellipsis (contract implementation note: truncate
/// at 120px).
const MAX_TAB_LABEL_WIDTH: f32 = 120.0;

/// One open project (workspace), mirroring the React `ProjectItem` (`app/src/App.tsx`). Mapped from a
/// backend `Workspace { id, name, created_at, updated_at }` row, keeping only the two fields the tab
/// strip needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectItem {
    /// Stable workspace id (the `active_project_id` value and the `/workspaces/:id/...` path segment).
    pub id: String,
    /// Display name shown on the tab.
    pub name: String,
}

impl ProjectItem {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
        }
    }
}

/// Stable out-of-process author_id for a single project tab (`project-tab-{id}`).
pub fn project_tab_author_id(project_id: &str) -> String {
    format!("project-tab-{project_id}")
}

/// Stable `egui::Id` for a single project tab. Derived from the author_id string (dynamic count) so a
/// tab's id is stable for a given project id across frames while staying clear of the fixed id band.
fn project_tab_egui_id(project_id: &str) -> egui::Id {
    egui::Id::new(project_tab_author_id(project_id))
}

/// The fetch state of the workspace list, so the strip can show a loading/error affordance without
/// blocking the render loop (red-team: a `/workspaces` timeout must never stall the shell).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum FetchState {
    /// No fetch has resolved yet; the strip shows whatever projects it currently has (possibly none).
    #[default]
    Idle,
    /// A fetch is in flight (a background task is running).
    Loading,
    /// The last fetch failed; the PREVIOUS project list is retained and this message is shown inline.
    Error(String),
}

/// The top project-tab strip widget + its state.
///
/// Owns the ordered project list, the active project id, and the last-fetch state. It does NOT own
/// pane layout — switching a project is reported to the caller ([`crate::app::HandshakeApp`]) which
/// performs the MT-009 save-old / load-new layout transition.
#[derive(Debug, Clone)]
pub struct ProjectTabBar {
    /// The open projects, in display order.
    projects: Vec<ProjectItem>,
    /// The id of the active project (the one whose layout the shell currently shows).
    active_id: String,
    /// Fetch state of the last `/workspaces` call.
    fetch_state: FetchState,
}

impl ProjectTabBar {
    /// Build a tab bar from a project list and the active project id.
    pub fn new(projects: Vec<ProjectItem>, active_id: impl Into<String>) -> Self {
        Self {
            projects,
            active_id: active_id.into(),
            fetch_state: FetchState::Idle,
        }
    }

    /// Read-only view of the current project list.
    pub fn projects(&self) -> &[ProjectItem] {
        &self.projects
    }

    /// The active project id.
    pub fn active_id(&self) -> &str {
        &self.active_id
    }

    /// Set the active project id (called by the app after a successful switch so the highlight tracks
    /// the shell's `active_project_id`).
    pub fn set_active_id(&mut self, id: impl Into<String>) {
        self.active_id = id.into();
    }

    /// The current fetch state.
    pub fn fetch_state(&self) -> &FetchState {
        &self.fetch_state
    }

    /// Mark that a workspace fetch is in flight.
    pub fn set_loading(&mut self) {
        self.fetch_state = FetchState::Loading;
    }

    /// Apply a fetched workspace list (success path). On an empty result the previous list is replaced
    /// by an empty list so the strip renders the "No projects" placeholder. The active id is preserved
    /// if it still exists in the new list, otherwise it falls back to the first project (so the
    /// highlight is never orphaned).
    pub fn apply_fetched(&mut self, projects: Vec<ProjectItem>) {
        let active_still_present = projects.iter().any(|p| p.id == self.active_id);
        self.projects = projects;
        if !active_still_present {
            if let Some(first) = self.projects.first() {
                self.active_id = first.id.clone();
            }
        }
        self.fetch_state = FetchState::Idle;
    }

    /// Record a fetch failure. The PREVIOUS project list is retained (red-team control: a transient
    /// `/workspaces` error must not blank the strip), and an inline error label is shown.
    pub fn apply_fetch_error(&mut self, message: impl Into<String>) {
        self.fetch_state = FetchState::Error(message.into());
    }

    /// Render the project-tab strip into `ui` and return `Some(project_id)` when the operator clicked a
    /// project tab that is NOT the active one (a switch request). Returns `None` when nothing changed
    /// or the active tab was re-clicked.
    ///
    /// Rendering:
    /// - a horizontally-scrollable row of slim tabs (one per project), the active one filled with the
    ///   accent-soft color and underlined, inactive ones using the surface fill;
    /// - an inline error label after the tabs when the last fetch failed;
    /// - a single disabled "No projects" tab when the project list is empty (never a crash).
    ///
    /// AccessKit:
    /// - the strip container is a `Role::TabList` node with author_id `project-tabs`;
    /// - each tab is a `Role::Tab` node with author_id `project-tab-{id}` + `Action::Click`/
    ///   `Action::Focus`; the active tab is marked `selected`.
    pub fn show(&mut self, ui: &mut egui::Ui, colors: ProjectTabColors) -> Option<String> {
        let mut switch_to: Option<String> = None;

        // Paint the strip background across the full available width.
        let bar_rect = ui.available_rect_before_wrap();
        if ui.is_rect_visible(bar_rect) {
            ui.painter().rect_filled(bar_rect, 0.0, colors.bar_bg);
        }

        ui.horizontal(|ui| {
            egui::ScrollArea::horizontal()
                .id_salt("project-tab-scroll")
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if self.projects.is_empty() {
                            Self::render_placeholder(ui, colors);
                        } else {
                            for project in &self.projects {
                                let is_active = project.id == self.active_id;
                                if Self::render_tab(ui, project, is_active, colors) && !is_active {
                                    switch_to = Some(project.id.clone());
                                }
                            }
                        }
                        if let FetchState::Error(msg) = &self.fetch_state {
                            ui.add_space(8.0);
                            ui.colored_label(colors.error, format!("workspaces: {msg}"));
                        }
                    });
                });
        });

        // ── Live AccessKit: the strip container is a TabList ──────────────────────────────────────
        // Register the container id on its rect first so the node attaches under the correct parent,
        // then enrich it (Role::TabList + author_id). Sense::focusable so keyboard nav can land on it.
        let container_id = unsafe { egui::Id::from_high_entropy_bits(PROJECT_TABS_NODE_ID) };
        ui.interact(bar_rect, container_id, egui::Sense::focusable_noninteractive());
        ui.ctx().accesskit_node_builder(container_id, |node| {
            node.set_role(accesskit::Role::TabList);
            node.set_author_id(PROJECT_TABS_AUTHOR_ID.to_owned());
            node.set_label("Project tabs".to_owned());
        });

        switch_to
    }

    /// Render a single project tab as a real interactive egui widget and emit its `Role::Tab`
    /// AccessKit node. Returns `true` if it was clicked this frame.
    fn render_tab(
        ui: &mut egui::Ui,
        project: &ProjectItem,
        is_active: bool,
        colors: ProjectTabColors,
    ) -> bool {
        let tab_id = project_tab_egui_id(&project.id);

        // Truncate the label galley at MAX_TAB_LABEL_WIDTH with an ellipsis so a long project name
        // does not blow out the strip.
        let font = egui::FontId::proportional(13.0);
        let mut job = egui::text::LayoutJob::single_section(
            project.name.clone(),
            egui::TextFormat::simple(font, colors.text),
        );
        // Single-row truncation with an ellipsis: cap the width, allow one row, and let epaint append
        // the overflow character (…) when the name exceeds MAX_TAB_LABEL_WIDTH.
        job.wrap = egui::text::TextWrapping {
            max_width: MAX_TAB_LABEL_WIDTH,
            max_rows: 1,
            break_anywhere: false,
            overflow_character: Some('\u{2026}'),
        };
        let galley = ui.painter().layout_job(job);

        let pad = 8.0;
        let content_w = pad + galley.size().x + pad;
        let height = PROJECT_TAB_BAR_HEIGHT - 2.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(content_w, height), egui::Sense::hover());
        // Interact at the FIXED tab_id so the Response, its widget_info (Role/label/Action::Click),
        // the AccessKit bounding box, and the author_id all land on the SAME node (mirrors the MT-007
        // tab + close-button id discipline).
        let response = ui.interact(rect, tab_id, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let bg = if is_active {
                colors.active_bg
            } else if response.hovered() {
                colors.hover_bg
            } else {
                colors.inactive_bg
            };
            ui.painter().rect_filled(rect, 3.0, bg);
            let text_pos = egui::pos2(
                rect.left() + pad,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(text_pos, galley, colors.text);
            // Active-tab underline accent (paper-strip aesthetic): a thin accent bar along the bottom.
            if is_active {
                let underline = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - 2.0),
                    egui::pos2(rect.right(), rect.bottom()),
                );
                ui.painter().rect_filled(underline, 0.0, colors.accent);
            }
        }

        // AccessKit: emit the Tab node enriched with role + author_id + selected state. egui already
        // derived Action::Click/Action::Focus from Sense::click(), so we ADD the Tab role + identity
        // here (mirrors the MT-007 tab body emission).
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::Button,
                ui.is_enabled(),
                is_active,
                &project.name,
            )
        });
        ui.ctx().accesskit_node_builder(tab_id, |node| {
            node.set_role(accesskit::Role::Tab);
            node.set_author_id(project_tab_author_id(&project.id));
            node.set_label(project.name.clone());
            if is_active {
                node.set_selected(true);
            }
        });

        response.clicked()
    }

    /// Render the single disabled "No projects" placeholder tab (empty workspace list). It is a
    /// non-interactive `Role::Tab` label so the strip never renders empty and never crashes, and an
    /// out-of-process model can read that the project list is empty by the `project-tab-none`
    /// author_id. It carries NO click action, so it does not trip the interactive-naming gate.
    fn render_placeholder(ui: &mut egui::Ui, colors: ProjectTabColors) {
        let label = "No projects";
        let font = egui::FontId::proportional(13.0);
        let galley = ui
            .painter()
            .layout_no_wrap(label.to_owned(), font, colors.disabled_text);
        let pad = 8.0;
        let content_w = pad + galley.size().x + pad;
        let height = PROJECT_TAB_BAR_HEIGHT - 2.0;
        let id = egui::Id::new(NO_PROJECTS_AUTHOR_ID);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(content_w, height), egui::Sense::hover());
        // Hover-only interaction (no click sense) so the placeholder is addressable/labelled but is
        // NOT an interactive control (the gate only flags clickable/focusable nodes without an id).
        ui.interact(rect, id, egui::Sense::hover());
        if ui.is_rect_visible(rect) {
            let text_pos =
                egui::pos2(rect.left() + pad, rect.center().y - galley.size().y * 0.5);
            ui.painter().galley(text_pos, galley, colors.disabled_text);
        }
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(accesskit::Role::Tab);
            node.set_author_id(NO_PROJECTS_AUTHOR_ID.to_owned());
            node.set_label(label.to_owned());
            node.set_disabled();
        });
    }
}

/// Colors the project-tab strip paints with, sourced from the active theme tokens by the caller so the
/// strip never reads egui's generic visuals (mirrors `tab_bar::TabBarColors` / `split_layout`).
#[derive(Debug, Clone, Copy)]
pub struct ProjectTabColors {
    /// Background fill of the whole strip and of inactive tabs.
    pub bar_bg: egui::Color32,
    /// Background fill of the active tab.
    pub active_bg: egui::Color32,
    /// Background fill of an inactive tab.
    pub inactive_bg: egui::Color32,
    /// Background fill of a hovered (inactive) tab.
    pub hover_bg: egui::Color32,
    /// Label text color.
    pub text: egui::Color32,
    /// Disabled placeholder text color.
    pub disabled_text: egui::Color32,
    /// Active-tab underline accent.
    pub accent: egui::Color32,
    /// Inline fetch-error label color.
    pub error: egui::Color32,
}

/// Fetch the open workspaces from the backend (`GET /workspaces`) and map each `Workspace { id, name,
/// created_at, updated_at }` row to a [`ProjectItem`]. Reuses the EXISTING handshake_core backend over
/// its real HTTP API (the native app never rebuilds backend logic) and deserializes via
/// `serde_json::Value` so it adds no dependency on the `handshake_core` crate's types — exactly the
/// pattern `backend_client::fetch_health` uses for `GET /health`.
///
/// Spawned to a background task by the app (NOT called on the egui UI thread) so a slow/absent backend
/// never stalls the render loop (red-team: a `/workspaces` timeout must not block the shell).
pub async fn fetch_workspaces(base_url: &str) -> Result<Vec<ProjectItem>, AppError> {
    let url = format!("{base_url}/workspaces");
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| AppError::Http(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(AppError::Http(format!("non-success status {}", resp.status())));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Parse(e.to_string()))?;
    // The endpoint returns a JSON array of workspace objects. Map each to a ProjectItem, skipping any
    // row missing an id (a malformed row must not panic the parse).
    let arr = v
        .as_array()
        .ok_or_else(|| AppError::Parse("expected a JSON array of workspaces".to_owned()))?;
    let projects = arr
        .iter()
        .filter_map(|row| {
            let id = row.get("id").and_then(|x| x.as_str())?;
            let name = row
                .get("name")
                .and_then(|x| x.as_str())
                .unwrap_or(id)
                .to_owned();
            Some(ProjectItem::new(id, name))
        })
        .collect();
    Ok(projects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn projects() -> Vec<ProjectItem> {
        vec![ProjectItem::new("a", "Alpha"), ProjectItem::new("b", "Beta")]
    }

    #[test]
    fn author_id_format_matches_contract() {
        assert_eq!(project_tab_author_id("a"), "project-tab-a");
        assert_eq!(project_tab_author_id("ws-123"), "project-tab-ws-123");
    }

    #[test]
    fn apply_fetched_empty_clears_list_for_placeholder() {
        let mut bar = ProjectTabBar::new(projects(), "a");
        bar.apply_fetched(Vec::new());
        assert!(bar.projects().is_empty(), "empty fetch -> placeholder state");
        assert_eq!(bar.fetch_state(), &FetchState::Idle);
    }

    #[test]
    fn apply_fetched_preserves_active_when_present() {
        let mut bar = ProjectTabBar::new(projects(), "b");
        bar.apply_fetched(vec![ProjectItem::new("b", "Beta"), ProjectItem::new("c", "Gamma")]);
        assert_eq!(bar.active_id(), "b", "active id preserved when still present");
    }

    #[test]
    fn apply_fetched_falls_back_to_first_when_active_gone() {
        let mut bar = ProjectTabBar::new(projects(), "a");
        bar.apply_fetched(vec![ProjectItem::new("c", "Gamma")]);
        assert_eq!(bar.active_id(), "c", "orphaned active id falls back to first project");
    }

    #[test]
    fn fetch_error_retains_previous_list() {
        let mut bar = ProjectTabBar::new(projects(), "a");
        bar.apply_fetch_error("connection refused");
        assert_eq!(bar.projects().len(), 2, "previous list retained on error");
        assert!(matches!(bar.fetch_state(), FetchState::Error(_)));
    }

    #[test]
    fn project_tabs_node_id_is_in_fresh_band() {
        // The container id (50) sits in the fresh 50..59 band: above scrollbars (40..43), below the
        // tab-bar containers (60..63), and strictly below the pane id base (100). Disjoint from every
        // other fixed id (10/20/21/30/31). The full collision proof is in accessibility::registry.
        assert_eq!(PROJECT_TABS_NODE_ID, 50);
        const { assert!(PROJECT_TABS_NODE_ID < crate::accessibility::PANE_NODE_ID_BASE) };
        for fixed in [10_u64, 20, 21, 30, 31, 40, 41, 42, 43, 60, 61, 62, 63] {
            assert_ne!(PROJECT_TABS_NODE_ID, fixed, "container id collides with {fixed}");
        }
    }
}
