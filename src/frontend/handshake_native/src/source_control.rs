//! Native source-control change-list surface (WP-KERNEL-011 MT-021, Surface 7).
//!
//! ## What this is
//!
//! A real, reusable native widget that renders the backend's source-control status as a list of
//! change rows, attaches the MT-021 [`source_control_context_items`] right-click menu to each row, and
//! dispatches the confirmed item to a typed [`SourceControlEvent`] the host applies (stage/unstage via
//! the verified backend, diff/blame display, copy path). It is the native peer of the React
//! `SourceControlPanel.tsx` change list — NOT a placeholder: every row carries a stable AccessKit
//! `Role::TreeItem` + `author_id` (`scm_row_{path_safe}`), the menu paints via the MT-019 hardened
//! popup, and the enable state comes from the real `StatusEntry { index, worktree }` flags.
//!
//! ## Backend (verified, NOT assumed)
//!
//! The stage/unstage/diff/blame calls route through [`crate::backend_client::SourceControlClient`],
//! whose URLs were verified READ-ONLY against `src/backend/handshake_core` (routes mounted at
//! `/source-control/*`, no `/api` prefix; discard is `confirmed`-gated, 409 when not confirmed). This
//! widget never blocks the render thread: a write/diff/blame is dispatched off-thread and the result is
//! drained from a delivery cell next frame (HBR-QUIET), the same pattern the MT-020 rename uses.
//!
//! ## Scope honesty
//!
//! WP-011 has no `PaneType` factory wired to this widget yet (the `SourceControl` pane still renders
//! the placeholder). This module is the change-list surface + its menu; mounting it as the live
//! `SourceControl` pane factory is the owning pane-content WP's job. The widget is fully driveable
//! (and proven LIVE) standalone via egui_kittest, so the menu behavior is real and tested now.

use egui::accesskit;

use crate::backend_client::ScmDiffScope;
use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{
    source_control_action_for_id, source_control_context_items, ScmRowMenuAction, ScmRowState,
};

/// One change row rendered by the panel: its path and the worktree/index change flags (mirrors the
/// verified backend `StatusEntry { index: Option<StatusCode>, worktree: Option<StatusCode> }` — a flag
/// is `true` iff the corresponding `Option` is `Some`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeRow {
    pub path: String,
    pub has_worktree: bool,
    pub has_index: bool,
}

impl ChangeRow {
    pub fn new(path: impl Into<String>, has_worktree: bool, has_index: bool) -> Self {
        Self {
            path: path.into(),
            has_worktree,
            has_index,
        }
    }

    fn state(&self) -> ScmRowState {
        ScmRowState {
            path: self.path.clone(),
            has_worktree: self.has_worktree,
            has_index: self.has_index,
        }
    }

    /// A short status badge (`M`/`+`/`S`) for the row prefix, so the change kind is visible.
    fn badge(&self) -> &'static str {
        match (self.has_index, self.has_worktree) {
            (true, true) => "MS",
            (true, false) => "S",
            (false, true) => "M",
            (false, false) => "·",
        }
    }
}

/// The typed event a confirmed source-control row menu produces, for the host to apply. The backend
/// writes (stage/unstage) carry the path; the displays (diff/blame) carry path + scope; copy carries
/// the path. Discard/commit are V1 stubs (their menu items are disabled), so they have no variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceControlEvent {
    /// `POST /source-control/stage` `{repo_path, paths:[path]}`.
    Stage { path: String },
    /// `POST /source-control/unstage` `{repo_path, paths:[path]}`.
    Unstage { path: String },
    /// `GET /source-control/diff?scope=…` then display in the diff area.
    Diff { path: String, scope: ScmDiffScope },
    /// `GET /source-control/blame` then display in the detail area.
    Blame { path: String },
    /// Copy the path to the clipboard.
    CopyPath { path: String },
}

/// Colors for the change list, sourced from the active theme by the host.
#[derive(Debug, Clone, Copy)]
pub struct SourceControlColors {
    pub row_bg: egui::Color32,
    pub row_hover_bg: egui::Color32,
    pub row_text: egui::Color32,
    pub badge_text: egui::Color32,
}

/// The native source-control change-list panel: a list of change rows, each right-clickable for the
/// MT-021 context menu. Holds the row list + the current diff/blame display text (updated when a
/// diff/blame action's off-thread result arrives). Stateless rendering: the host owns the data + the
/// backend client and applies the returned event.
#[derive(Debug, Clone, Default)]
pub struct SourceControlPanel {
    /// The change rows (from the backend status). The host refreshes these after a write.
    pub rows: Vec<ChangeRow>,
    /// The diff/blame text last displayed (set by the host when an off-thread diff/blame arrives).
    pub display_text: String,
}

impl SourceControlPanel {
    pub fn new(rows: Vec<ChangeRow>) -> Self {
        Self {
            rows,
            display_text: String::new(),
        }
    }

    /// Render the change list + the diff/blame display area. Returns the typed event a confirmed
    /// right-click menu item produced this frame (at most one — egui keeps one menu open at a time).
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        colors: SourceControlColors,
    ) -> Option<SourceControlEvent> {
        let mut event = None;
        ui.label("Source Control");
        for (idx, row) in self.rows.iter().enumerate() {
            if let Some(e) = self.row(ui, idx, row, colors) {
                event = Some(e);
            }
        }
        if !self.display_text.is_empty() {
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("scm-diff-display")
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(&self.display_text).monospace(),
                        )
                        .wrap(),
                    );
                });
        }
        event
    }

    /// Render one change row as a `Role::TreeItem` with a stable `scm_row_{path_safe}` author_id, and
    /// attach the MT-021 menu. Returns the menu-driven event when an enabled item is confirmed.
    fn row(
        &self,
        ui: &mut egui::Ui,
        idx: usize,
        row: &ChangeRow,
        colors: SourceControlColors,
    ) -> Option<SourceControlEvent> {
        let author_id = scm_row_author_id(&row.path);
        let id = egui::Id::new(&author_id);
        let label = format!("{}  {}", row.badge(), row.path);
        let height = 20.0;
        let resp = ui
            .horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), height),
                    egui::Sense::hover(),
                );
                let resp = ui.interact(rect, id, egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    let bg = if resp.hovered() { colors.row_hover_bg } else { colors.row_bg };
                    ui.painter().rect_filled(rect, 3.0, bg);
                    let galley = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::monospace(12.0),
                        colors.row_text,
                    );
                    let pos = egui::pos2(rect.left() + 6.0, rect.center().y - galley.size().y * 0.5);
                    ui.painter().galley(pos, galley, colors.row_text);
                }
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
                });
                ui.ctx().accesskit_node_builder(id, |node| {
                    node.set_role(accesskit::Role::TreeItem);
                    node.set_author_id(author_id.clone());
                    node.set_label(label.clone());
                });
                resp
            })
            .inner;
        let _ = idx;

        let mut event = None;
        let menu = ContextMenu::new("scm").items(source_control_context_items(&row.state()));
        if let Some(confirmed_id) = menu.show_on(&resp) {
            if let Some(action) = source_control_action_for_id(confirmed_id, &row.state()) {
                event = Some(self.event_for(action, row));
            }
        }
        // Shift+F10 keyboard-open parity (egui 0.33 has no dedicated Menu/ContextMenu key).
        if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
            crate::context_menu::request_open(ui.ctx(), resp.id, resp.rect.left_bottom());
        }
        event
    }

    /// Map a typed menu action to the surface event the host applies.
    fn event_for(&self, action: ScmRowMenuAction, row: &ChangeRow) -> SourceControlEvent {
        match action {
            ScmRowMenuAction::Stage => SourceControlEvent::Stage { path: row.path.clone() },
            ScmRowMenuAction::Unstage => SourceControlEvent::Unstage { path: row.path.clone() },
            ScmRowMenuAction::DiffWorktree => SourceControlEvent::Diff {
                path: row.path.clone(),
                scope: ScmDiffScope::Worktree,
            },
            ScmRowMenuAction::DiffStaged => SourceControlEvent::Diff {
                path: row.path.clone(),
                scope: ScmDiffScope::Staged,
            },
            ScmRowMenuAction::Blame => SourceControlEvent::Blame { path: row.path.clone() },
            ScmRowMenuAction::CopyPath => SourceControlEvent::CopyPath { path: row.path.clone() },
        }
    }
}

/// Stable AccessKit author_id for a change row: `scm_row_{path_safe}`, where `path_safe` is the path
/// slugified to `[a-z0-9-]` (so a path with slashes/spaces never produces a malformed id). Mirrors the
/// project-tree `stable_part` convention.
pub fn scm_row_author_id(path: &str) -> String {
    format!("scm_row_{}", crate::project_tree::stable_part(path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_reflects_change_flags() {
        assert_eq!(ChangeRow::new("a", true, false).badge(), "M");
        assert_eq!(ChangeRow::new("a", false, true).badge(), "S");
        assert_eq!(ChangeRow::new("a", true, true).badge(), "MS");
    }

    #[test]
    fn author_id_is_slug_safe() {
        let id = scm_row_author_id("src/sub dir/x.rs");
        assert!(id.starts_with("scm_row_"));
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    }

    #[test]
    fn stage_event_carries_path() {
        let panel = SourceControlPanel::new(vec![ChangeRow::new("x.rs", true, false)]);
        let row = &panel.rows[0];
        assert_eq!(
            panel.event_for(ScmRowMenuAction::Stage, row),
            SourceControlEvent::Stage { path: "x.rs".to_owned() },
        );
        assert_eq!(
            panel.event_for(ScmRowMenuAction::DiffWorktree, row),
            SourceControlEvent::Diff { path: "x.rs".to_owned(), scope: ScmDiffScope::Worktree },
        );
    }
}
