//! Native drawer-item surface (WP-KERNEL-011 MT-021, Surface 9).
//!
//! ## What this is
//!
//! A real, reusable native widget that renders bottom-stash-shelf drawer-item cards, attaches the
//! MT-021 [`drawer_item_context_items`] right-click menu to each, and dispatches the confirmed item to a
//! typed [`DrawerEvent`] the host applies. Each card is a `Role::ListItem` carrying a stable
//! `drawer_item_{item_id}` author_id. The `Send to Pane...` submenu children are built DYNAMICALLY from
//! the open panes at right-click time (`drawer.send_to_pane.{pane_id}` child ids; the parent id stays
//! the stable `drawer.send_to_pane`).
//!
//! ## Scope honesty (MT-023 dependency)
//!
//! The FULL bottom stash shelf (its container, drag, typed cards) is **MT-023 / C6**, which is not part
//! of WP-011. This MT-021 module authors the per-item context menu for C5 completeness and renders the
//! cards on a minimal item surface so the menu is real + proven LIVE now; mounting it inside the MT-023
//! bottom-drawer container is MT-023's job. `attach_evidence` / `convert_artifact` are V1 stubs
//! (disabled menu items — no evidence/artifact endpoint wired yet).

use egui::accesskit;

use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{
    drawer_item_action_for_id, drawer_item_context_items, DrawerItemMenuAction, DrawerItemState,
};

/// The typed event a confirmed drawer-item menu produces. Pin carries the NEW target value; SendToPane
/// carries the chosen pane id (a dynamic submenu child). Stubs (attach_evidence/convert_artifact) have
/// no variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerEvent {
    /// Minimize the card to an icon in the shelf.
    Stow { item_id: String },
    /// Toggle the card's pinned flag (`target` is the NEW value).
    SetPinned { item_id: String, target: bool },
    /// Open the card content as a full pane.
    Promote { item_id: String },
    /// Move the card content to the chosen open pane as a new tab.
    SendToPane { item_id: String, pane_id: String },
    /// Copy the card text into the focused prompt/editor.
    CopyToPrompt { item_id: String, text: String },
    /// Remove the card from the shelf.
    Discard { item_id: String },
}

/// One drawer card rendered by the surface: its cached state + display label + text content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrawerCard {
    pub state: DrawerItemState,
    pub label: String,
    pub text_content: String,
}

impl DrawerCard {
    pub fn new(
        state: DrawerItemState,
        label: impl Into<String>,
        text_content: impl Into<String>,
    ) -> Self {
        Self {
            state,
            label: label.into(),
            text_content: text_content.into(),
        }
    }
}

/// Colors for the drawer cards, sourced from the active theme by the host.
#[derive(Debug, Clone, Copy)]
pub struct DrawerColors {
    pub card_bg: egui::Color32,
    pub card_hover_bg: egui::Color32,
    pub card_text: egui::Color32,
}

/// The native drawer-item surface: a set of cards, each right-clickable for the MT-021 menu. The
/// `open_panes` list (`(pane_id, display_label)`) feeds the dynamic `Send to Pane...` submenu.
#[derive(Debug, Clone, Default)]
pub struct DrawerSurface {
    pub cards: Vec<DrawerCard>,
    pub open_panes: Vec<(String, String)>,
}

impl DrawerSurface {
    pub fn new(cards: Vec<DrawerCard>, open_panes: Vec<(String, String)>) -> Self {
        Self { cards, open_panes }
    }

    /// Render the cards; return the typed event a confirmed right-click menu item produced this frame.
    pub fn show(&self, ui: &mut egui::Ui, colors: DrawerColors) -> Option<DrawerEvent> {
        let mut event = None;
        ui.label("Stash");
        for card in &self.cards {
            if let Some(e) = self.card(ui, card, colors) {
                event = Some(e);
            }
        }
        event
    }

    fn card(
        &self,
        ui: &mut egui::Ui,
        card: &DrawerCard,
        colors: DrawerColors,
    ) -> Option<DrawerEvent> {
        let author_id = drawer_item_author_id(&card.state.item_id);
        let id = egui::Id::new(&author_id);
        let label = card.label.clone();
        let resp = ui
            .horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().min(200.0), 26.0),
                    egui::Sense::hover(),
                );
                let resp = ui.interact(rect, id, egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    let bg = if resp.hovered() { colors.card_hover_bg } else { colors.card_bg };
                    ui.painter().rect_filled(rect, 4.0, bg);
                    let galley = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::proportional(13.0),
                        colors.card_text,
                    );
                    let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5);
                    ui.painter().galley(pos, galley, colors.card_text);
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

        let mut event = None;
        let menu = ContextMenu::new("drawer")
            .items(drawer_item_context_items(&card.state, &self.open_panes));
        if let Some(confirmed_id) = menu.show_on(&resp) {
            if let Some(action) = drawer_item_action_for_id(confirmed_id, &card.state) {
                event = Some(self.event_for(action, card));
            }
        }
        if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
            crate::context_menu::request_open(ui.ctx(), resp.id, resp.rect.left_bottom());
        }
        event
    }

    fn event_for(&self, action: DrawerItemMenuAction, card: &DrawerCard) -> DrawerEvent {
        let item_id = card.state.item_id.clone();
        match action {
            DrawerItemMenuAction::Stow => DrawerEvent::Stow { item_id },
            DrawerItemMenuAction::TogglePin { target } => DrawerEvent::SetPinned { item_id, target },
            DrawerItemMenuAction::Promote => DrawerEvent::Promote { item_id },
            DrawerItemMenuAction::SendToPane { pane_id } => {
                // Red-team send_to_pane control: only target a pane that STILL exists at confirm time;
                // if the pane closed between right-click and selection, fall back to no event (the host
                // logs + opens in the focused pane). Here we verify membership against the live list.
                if self.open_panes.iter().any(|(id, _)| id == &pane_id) {
                    DrawerEvent::SendToPane { item_id, pane_id }
                } else {
                    tracing::warn!(
                        "drawer send_to_pane: target pane {pane_id} closed before completion; \
                         falling back to focused pane"
                    );
                    DrawerEvent::Promote { item_id }
                }
            }
            DrawerItemMenuAction::CopyToPrompt => DrawerEvent::CopyToPrompt {
                item_id,
                text: card.text_content.clone(),
            },
            DrawerItemMenuAction::Discard => DrawerEvent::Discard { item_id },
        }
    }
}

/// Stable AccessKit author_id for a drawer card: `drawer_item_{item_id}` (slug-safe).
pub fn drawer_item_author_id(item_id: &str) -> String {
    format!("drawer_item_{}", crate::project_tree::stable_part(item_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface() -> DrawerSurface {
        DrawerSurface::new(
            vec![DrawerCard::new(
                DrawerItemState {
                    item_id: "card-1".to_owned(),
                    pinned: false,
                },
                "Note",
                "the card text",
            )],
            vec![("pane-a".to_owned(), "Workspace".to_owned())],
        )
    }

    #[test]
    fn pin_event_flips_target() {
        let s = surface();
        assert_eq!(
            s.event_for(DrawerItemMenuAction::TogglePin { target: true }, &s.cards[0]),
            DrawerEvent::SetPinned { item_id: "card-1".to_owned(), target: true },
        );
    }

    #[test]
    fn send_to_existing_pane_targets_it() {
        let s = surface();
        assert_eq!(
            s.event_for(
                DrawerItemMenuAction::SendToPane { pane_id: "pane-a".to_owned() },
                &s.cards[0],
            ),
            DrawerEvent::SendToPane {
                item_id: "card-1".to_owned(),
                pane_id: "pane-a".to_owned(),
            },
        );
    }

    #[test]
    fn send_to_closed_pane_falls_back_to_promote() {
        let s = surface();
        // pane-z is not in open_panes → the red-team fallback fires (no event to a dead pane).
        assert_eq!(
            s.event_for(
                DrawerItemMenuAction::SendToPane { pane_id: "pane-z".to_owned() },
                &s.cards[0],
            ),
            DrawerEvent::Promote { item_id: "card-1".to_owned() },
        );
    }

    #[test]
    fn copy_to_prompt_carries_text() {
        let s = surface();
        assert_eq!(
            s.event_for(DrawerItemMenuAction::CopyToPrompt, &s.cards[0]),
            DrawerEvent::CopyToPrompt {
                item_id: "card-1".to_owned(),
                text: "the card text".to_owned(),
            },
        );
    }
}
