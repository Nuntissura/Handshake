//! Native canvas-board node/card surface (WP-KERNEL-011 MT-021, Surface 6).
//!
//! ## What this is
//!
//! A real, reusable native widget that renders canvas placements (block previews + text cards),
//! attaches the MT-021 [`canvas_node_context_items`] right-click menu to each, and dispatches the
//! confirmed item to a typed [`CanvasBoardEvent`] the host applies. Native peer of the React
//! `LoomCanvasBoard.tsx`. Each placement is a `Role::TreeItem` (egui has no canvas-node role) carrying a
//! stable `canvas_node_{placement_id}` author_id.
//!
//! ## Backend (verified)
//!
//! `move_to_front`/`move_to_back` PATCH `z_index` and `remove` DELETEs the placement via the VERIFIED
//! `PATCH/DELETE /workspaces/:ws/loom/canvas-placements/:placement_id` routes; `remove_edges` DELETEs
//! only VISUAL-only edges via `DELETE /workspaces/:ws/loom/canvas-visual-edges/:edge_id` (NEVER a
//! semantic Loom edge — red-team control), all through [`crate::backend_client::CanvasClient`] off the
//! UI thread. `connect_to`/`add_visual_edge`/`delete_block` are V1 stubs (disabled menu items).
//!
//! ## Scope honesty
//!
//! WP-011 has no native canvas pane yet; mounting this as a live pane is the owning canvas-pane WP's
//! job. The node + its menu are real and proven LIVE standalone via egui_kittest.

use egui::accesskit;

use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{
    canvas_node_action_for_id, canvas_node_context_items, CanvasNodeMenuAction, CanvasNodeState,
};

/// The typed event a confirmed canvas-node menu produces. Stubs (connect_to/add_visual_edge/
/// delete_block) have no variant. `RemoveEdges` carries the placement id; the host enumerates the
/// board's VISUAL edges for that placement and DELETEs each via `CanvasClient::remove_visual_edge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanvasBoardEvent {
    /// Open the placed block in a tab (block placements only).
    OpenBlock {
        placement_id: String,
        block_id: String,
    },
    /// Enter inline text-edit on the card (card placements only).
    EditCard { placement_id: String },
    /// Remove all VISUAL-only edges connected to this placement.
    RemoveEdges { placement_id: String },
    /// PATCH `z_index` to the front of the board.
    MoveToFront { placement_id: String },
    /// PATCH `z_index` to the back of the board.
    MoveToBack { placement_id: String },
    /// Copy the placed block id (block placements only).
    CopyBlockId { block_id: String },
    /// DELETE the placement (the canvas reference), NOT the underlying block.
    Remove { placement_id: String },
}

/// One canvas placement rendered by the board: its cached state, display label, and (for a block) the
/// placed block id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    pub state: CanvasNodeState,
    pub label: String,
    /// `Some` for a block placement (the placed LoomBlock id); `None` for a text-only card.
    pub block_id: Option<String>,
}

impl Placement {
    pub fn new(state: CanvasNodeState, label: impl Into<String>, block_id: Option<String>) -> Self {
        Self {
            state,
            label: label.into(),
            block_id,
        }
    }
}

/// Colors for the canvas placements, sourced from the active theme by the host.
#[derive(Debug, Clone, Copy)]
pub struct CanvasBoardColors {
    pub card_bg: egui::Color32,
    pub card_hover_bg: egui::Color32,
    pub card_text: egui::Color32,
}

/// The native canvas-board surface: a set of placements, each right-clickable for the MT-021 menu.
#[derive(Debug, Clone, Default)]
pub struct CanvasBoardSurface {
    pub placements: Vec<Placement>,
}

impl CanvasBoardSurface {
    pub fn new(placements: Vec<Placement>) -> Self {
        Self { placements }
    }

    /// Render the placements; return the typed event a confirmed right-click menu item produced.
    pub fn show(&self, ui: &mut egui::Ui, colors: CanvasBoardColors) -> Option<CanvasBoardEvent> {
        let mut event = None;
        ui.label("Canvas");
        for placement in &self.placements {
            if let Some(e) = self.placement(ui, placement, colors) {
                event = Some(e);
            }
        }
        event
    }

    fn placement(
        &self,
        ui: &mut egui::Ui,
        placement: &Placement,
        colors: CanvasBoardColors,
    ) -> Option<CanvasBoardEvent> {
        let author_id = canvas_node_author_id(&placement.state.placement_id);
        let id = egui::Id::new(&author_id);
        let label = placement.label.clone();
        let resp = ui
            .horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().min(220.0), 28.0),
                    egui::Sense::hover(),
                );
                let resp = ui.interact(rect, id, egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    let bg = if resp.hovered() {
                        colors.card_hover_bg
                    } else {
                        colors.card_bg
                    };
                    ui.painter().rect_filled(rect, 4.0, bg);
                    let galley = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::proportional(13.0),
                        colors.card_text,
                    );
                    let pos =
                        egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5);
                    ui.painter().galley(pos, galley, colors.card_text);
                }
                resp.widget_info(|| {
                    egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &label)
                });
                ui.ctx().accesskit_node_builder(id, |node_b| {
                    node_b.set_role(accesskit::Role::TreeItem);
                    node_b.set_author_id(author_id.clone());
                    node_b.set_label(label.clone());
                });
                resp
            })
            .inner;

        let mut event = None;
        let menu = ContextMenu::new("canvas").items(canvas_node_context_items(&placement.state));
        if let Some(confirmed_id) = menu.show_on(&resp) {
            if let Some(action) = canvas_node_action_for_id(confirmed_id, &placement.state) {
                if let Some(e) = self.event_for(action, placement) {
                    event = Some(e);
                }
            }
        }
        if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
            crate::context_menu::request_open(ui.ctx(), resp.id, resp.rect.left_bottom());
        }
        event
    }

    /// Map a typed action to the surface event. `OpenBlock`/`CopyBlockId` need the block id — if a card
    /// placement (no block id) somehow produced them, return `None` (defensive; the mapper already
    /// gates those on kind).
    fn event_for(
        &self,
        action: CanvasNodeMenuAction,
        placement: &Placement,
    ) -> Option<CanvasBoardEvent> {
        let placement_id = placement.state.placement_id.clone();
        Some(match action {
            CanvasNodeMenuAction::OpenBlock => {
                let block_id = placement.block_id.clone()?;
                CanvasBoardEvent::OpenBlock {
                    placement_id,
                    block_id,
                }
            }
            CanvasNodeMenuAction::EditCard => CanvasBoardEvent::EditCard { placement_id },
            CanvasNodeMenuAction::RemoveEdges => CanvasBoardEvent::RemoveEdges { placement_id },
            CanvasNodeMenuAction::MoveToFront => CanvasBoardEvent::MoveToFront { placement_id },
            CanvasNodeMenuAction::MoveToBack => CanvasBoardEvent::MoveToBack { placement_id },
            CanvasNodeMenuAction::CopyBlockId => {
                let block_id = placement.block_id.clone()?;
                CanvasBoardEvent::CopyBlockId { block_id }
            }
            CanvasNodeMenuAction::Remove => CanvasBoardEvent::Remove { placement_id },
        })
    }
}

/// Stable AccessKit author_id for a placement: `canvas_node_{placement_id}` (slug-safe).
pub fn canvas_node_author_id(placement_id: &str) -> String {
    format!(
        "canvas_node_{}",
        crate::project_tree::stable_part(placement_id)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context_menu_surfaces::CanvasNodeKind;

    fn block_placement() -> Placement {
        Placement::new(
            CanvasNodeState {
                placement_id: "pl-1".to_owned(),
                kind: CanvasNodeKind::Block,
                has_visual_edges: true,
            },
            "Block card",
            Some("blk-9".to_owned()),
        )
    }

    #[test]
    fn remove_event_carries_placement_id() {
        let surface = CanvasBoardSurface::new(vec![block_placement()]);
        assert_eq!(
            surface.event_for(CanvasNodeMenuAction::Remove, &surface.placements[0]),
            Some(CanvasBoardEvent::Remove {
                placement_id: "pl-1".to_owned()
            }),
        );
    }

    #[test]
    fn open_block_carries_block_id() {
        let surface = CanvasBoardSurface::new(vec![block_placement()]);
        assert_eq!(
            surface.event_for(CanvasNodeMenuAction::OpenBlock, &surface.placements[0]),
            Some(CanvasBoardEvent::OpenBlock {
                placement_id: "pl-1".to_owned(),
                block_id: "blk-9".to_owned(),
            }),
        );
    }

    #[test]
    fn move_to_front_carries_placement_id() {
        let surface = CanvasBoardSurface::new(vec![block_placement()]);
        assert_eq!(
            surface.event_for(CanvasNodeMenuAction::MoveToFront, &surface.placements[0]),
            Some(CanvasBoardEvent::MoveToFront {
                placement_id: "pl-1".to_owned()
            }),
        );
    }
}
