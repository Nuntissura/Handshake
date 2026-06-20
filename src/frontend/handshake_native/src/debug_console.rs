//! Native debug-console surface (WP-KERNEL-011 MT-021, Surface 8).
//!
//! ## What this is
//!
//! A real, reusable native widget that renders an in-memory list of debug-console entries (input /
//! result / error / output), attaches the MT-021 [`console_row_context_items`] right-click menu to each
//! row, and APPLIES the confirmed action directly to its own state (copy to clipboard, set the display
//! filter, clear the in-memory entries). Native peer of the React `DebugConsole.tsx`. Each row is a
//! `Role::ListItem` carrying a stable `console_row_{index}` author_id.
//!
//! ## State ownership (red-team console controls)
//!
//! - `clear` empties the IN-MEMORY display `entries` ONLY. It does NOT touch any persistent backend log
//!   — the console is a display buffer, not the authoritative store (documented on [`Self::clear`]).
//! - `filter` is a display-only flag; the render applies it before iterating entries, so clearing the
//!   filter (`Show All`) restores the full list without re-fetching.
//!
//! This widget owns + mutates its state in place (no host event needed for the in-widget actions),
//! which is why [`Self::show`] takes `&mut self`. Copy goes to the egui clipboard (`ctx.copy_text`), so
//! the acceptance assertion `ctx.output(|o| o.copied_text)` reads it back.

use egui::accesskit;

use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{
    console_row_action_for_id, console_row_context_items, ConsoleEntryKind, ConsoleRowMenuAction,
};

/// One debug-console entry: its kind and text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleEntry {
    pub kind: ConsoleEntryKind,
    pub text: String,
}

impl ConsoleEntry {
    pub fn new(kind: ConsoleEntryKind, text: impl Into<String>) -> Self {
        Self {
            kind,
            text: text.into(),
        }
    }
}

/// Colors for the console rows, sourced from the active theme by the host.
#[derive(Debug, Clone, Copy)]
pub struct DebugConsoleColors {
    pub row_bg: egui::Color32,
    pub row_hover_bg: egui::Color32,
    pub row_text: egui::Color32,
}

/// The native debug-console surface. Owns the in-memory entries + the display filter, and mutates them
/// in place from the right-click menu (copy / filter / clear).
#[derive(Debug, Clone, Default)]
pub struct DebugConsole {
    /// The in-memory display entries. `clear` empties this; it is NOT a persistent backend log.
    pub entries: Vec<ConsoleEntry>,
    /// The active display filter: `None` shows all, `Some(kind)` shows only that kind.
    pub filter: Option<ConsoleEntryKind>,
}

impl DebugConsole {
    pub fn new(entries: Vec<ConsoleEntry>) -> Self {
        Self {
            entries,
            filter: None,
        }
    }

    /// Clear the IN-MEMORY display entries ONLY. This does NOT delete any persistent backend log — the
    /// console is a display buffer, so the user loses only the on-screen history (red-team
    /// console.clear control). After this, `entries.len() == 0`.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Render the console rows (honoring the active filter), applying any confirmed right-click menu
    /// action to this console's own state in place. Takes `&mut self` because copy/filter/clear mutate
    /// the console (copy writes the egui clipboard; filter sets the flag; clear empties entries).
    pub fn show(&mut self, ui: &mut egui::Ui, colors: DebugConsoleColors) {
        ui.label("Debug Console");
        // Indices of the entries the active filter shows, in order. Rendered with their ORIGINAL index
        // so the author_id stays stable for a given entry across filter changes.
        let visible: Vec<usize> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| self.filter.is_none_or(|f| f == e.kind))
            .map(|(i, _)| i)
            .collect();

        let mut pending: Option<ConsoleRowMenuAction> = None;
        let mut pending_row: Option<usize> = None;
        for &i in &visible {
            let entry = &self.entries[i];
            if let Some(action) = Self::row(ui, i, entry, colors) {
                pending = Some(action);
                pending_row = Some(i);
            }
        }
        if let (Some(action), Some(row_idx)) = (pending, pending_row) {
            self.apply(ui.ctx(), action, row_idx);
        }
    }

    /// Render one console row as a `Role::ListItem` with a stable `console_row_{index}` author_id, and
    /// attach the MT-021 menu. Returns the confirmed action (the caller applies it to `self`).
    fn row(
        ui: &mut egui::Ui,
        index: usize,
        entry: &ConsoleEntry,
        colors: DebugConsoleColors,
    ) -> Option<ConsoleRowMenuAction> {
        let author_id = console_row_author_id(index);
        let id = egui::Id::new(&author_id);
        let label = entry.text.clone();
        let resp = ui
            .horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 18.0),
                    egui::Sense::hover(),
                );
                let resp = ui.interact(rect, id, egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    let bg = if resp.hovered() { colors.row_hover_bg } else { colors.row_bg };
                    ui.painter().rect_filled(rect, 2.0, bg);
                    let galley = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::monospace(12.0),
                        colors.row_text,
                    );
                    let pos = egui::pos2(rect.left() + 4.0, rect.center().y - galley.size().y * 0.5);
                    ui.painter().galley(pos, galley, colors.row_text);
                }
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
                });
                ui.ctx().accesskit_node_builder(id, |node| {
                    node.set_role(accesskit::Role::ListItem);
                    node.set_author_id(author_id.clone());
                    node.set_label(label.clone());
                });
                resp
            })
            .inner;

        let mut action = None;
        let menu = ContextMenu::new("console").items(console_row_context_items());
        if let Some(confirmed_id) = menu.show_on(&resp) {
            action = console_row_action_for_id(confirmed_id);
        }
        if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
            crate::context_menu::request_open(ui.ctx(), resp.id, resp.rect.left_bottom());
        }
        action
    }

    /// Apply a confirmed console action to this console's state: copy the row text / copy all / set the
    /// filter / clear the in-memory entries. `row_idx` is the original index of the right-clicked row.
    fn apply(&mut self, ctx: &egui::Context, action: ConsoleRowMenuAction, row_idx: usize) {
        match action {
            ConsoleRowMenuAction::CopyLine => {
                if let Some(entry) = self.entries.get(row_idx) {
                    ctx.copy_text(entry.text.clone());
                }
            }
            ConsoleRowMenuAction::CopyAll => {
                let all = self
                    .entries
                    .iter()
                    .map(|e| e.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                ctx.copy_text(all);
            }
            ConsoleRowMenuAction::SetFilter(filter) => {
                self.filter = filter;
            }
            ConsoleRowMenuAction::Clear => self.clear(),
        }
    }
}

/// Stable AccessKit author_id for a console row: `console_row_{index}`.
pub fn console_row_author_id(index: usize) -> String {
    format!("console_row_{index}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn console() -> DebugConsole {
        DebugConsole::new(vec![
            ConsoleEntry::new(ConsoleEntryKind::Input, "ls"),
            ConsoleEntry::new(ConsoleEntryKind::Output, "a b c"),
            ConsoleEntry::new(ConsoleEntryKind::Error, "boom"),
        ])
    }

    #[test]
    fn clear_empties_in_memory_entries() {
        let mut c = console();
        assert_eq!(c.entries.len(), 3);
        c.clear();
        assert_eq!(c.entries.len(), 0, "clear empties the in-memory display list");
    }

    #[test]
    fn apply_clear_via_action_empties_entries() {
        let ctx = egui::Context::default();
        let mut c = console();
        c.apply(&ctx, ConsoleRowMenuAction::Clear, 0);
        assert_eq!(c.entries.len(), 0);
    }

    #[test]
    fn apply_set_filter_sets_flag() {
        let ctx = egui::Context::default();
        let mut c = console();
        c.apply(&ctx, ConsoleRowMenuAction::SetFilter(Some(ConsoleEntryKind::Error)), 0);
        assert_eq!(c.filter, Some(ConsoleEntryKind::Error));
        c.apply(&ctx, ConsoleRowMenuAction::SetFilter(None), 0);
        assert_eq!(c.filter, None, "Show All clears the filter");
    }

    #[test]
    fn author_id_is_indexed() {
        assert_eq!(console_row_author_id(2), "console_row_2");
    }
}
