//! Native Loom-graph node surface (WP-KERNEL-011 MT-021, Surface 5).
//!
//! ## What this is
//!
//! A real, reusable native widget that renders Loom-graph nodes from their cached
//! [`crate::context_menu_surfaces::LoomNodeState`] (the freshest loaded block state — pinned/favorite
//! flags + whether the node has edges), attaches the MT-021 [`loom_node_context_items`] right-click
//! menu to each node, and dispatches the confirmed item to a typed [`LoomGraphEvent`] the host applies.
//! It is the native peer of the React Loom graph / `LoomBlockPanel.tsx` — NOT a placeholder: each node
//! is a `Role::TreeItem` (egui has no Graph-node role) carrying a stable `loom_node_{block_id}`
//! author_id, and the pin/favorite labels + toggle target come from the FRESH cached state so the
//! toggle always flips the right way (red-team stale-state control).
//!
//! ## Backend (verified)
//!
//! `pin`/`favorite`/`rename` route through the VERIFIED `PATCH /workspaces/:id/loom/blocks/:block_id`
//! endpoint via [`crate::backend_client::LoomBlockClient`] (the same client MT-020's explorer rename
//! uses), off the UI thread (HBR-QUIET). `connect`/`disconnect`/`delete` are V1 stubs (their menu
//! items are disabled — no edge-edit / confirm-dialog endpoint wired yet).
//!
//! ## Scope honesty
//!
//! WP-011 has no native graph-canvas pane yet (the React `LoomSearchV2Panel`/`LoomBlockPanel` are a
//! thin placeholder); mounting this as a live pane is the owning Loom-pane WP's job. The node + its
//! menu are real and proven LIVE standalone via egui_kittest.

use egui::accesskit;

use crate::context_menu::ContextMenu;
use crate::context_menu_surfaces::{
    loom_node_action_for_id, loom_node_context_items, LoomNodeMenuAction, LoomNodeState,
};

/// The typed event a confirmed Loom-node menu produces, for the host to apply. Pin/favorite carry the
/// NEW target value (computed from the fresh cached state). Stubs (connect/disconnect/delete) have no
/// variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoomGraphEvent {
    /// Open the block in a tab on the active pane.
    Open { block_id: String },
    /// Split right then open the block.
    OpenToSide { block_id: String },
    /// Rename the block via the verified PATCH `{title}`.
    Rename { block_id: String, current_title: String },
    /// PATCH `{pinned: target}` on the block.
    SetPinned { block_id: String, target: bool },
    /// PATCH `{favorite: target}` on the block.
    SetFavorite { block_id: String, target: bool },
    /// Copy the block id to the clipboard.
    CopyBlockId { block_id: String },
    /// Open / focus the LoomBlock panel pane for the block.
    RevealInPanel { block_id: String },
}

/// One graph node rendered by the surface: its cached state + display title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub state: LoomNodeState,
    pub title: String,
}

impl GraphNode {
    pub fn new(state: LoomNodeState, title: impl Into<String>) -> Self {
        Self {
            state,
            title: title.into(),
        }
    }
}

/// Colors for the graph nodes, sourced from the active theme by the host.
#[derive(Debug, Clone, Copy)]
pub struct LoomGraphColors {
    pub node_bg: egui::Color32,
    pub node_hover_bg: egui::Color32,
    pub node_text: egui::Color32,
}

/// The native Loom-graph node surface: a set of nodes, each right-clickable for the MT-021 menu.
#[derive(Debug, Clone, Default)]
pub struct LoomGraphSurface {
    pub nodes: Vec<GraphNode>,
}

impl LoomGraphSurface {
    pub fn new(nodes: Vec<GraphNode>) -> Self {
        Self { nodes }
    }

    /// Render the nodes; return the typed event a confirmed right-click menu item produced this frame.
    pub fn show(&self, ui: &mut egui::Ui, colors: LoomGraphColors) -> Option<LoomGraphEvent> {
        let mut event = None;
        ui.label("Loom Graph");
        for node in &self.nodes {
            if let Some(e) = self.node(ui, node, colors) {
                event = Some(e);
            }
        }
        event
    }

    fn node(
        &self,
        ui: &mut egui::Ui,
        node: &GraphNode,
        colors: LoomGraphColors,
    ) -> Option<LoomGraphEvent> {
        let author_id = loom_node_author_id(&node.state.block_id);
        let id = egui::Id::new(&author_id);
        let label = node.title.clone();
        let resp = ui
            .horizontal(|ui| {
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width().min(220.0), 24.0),
                    egui::Sense::hover(),
                );
                let resp = ui.interact(rect, id, egui::Sense::click());
                if ui.is_rect_visible(rect) {
                    let bg = if resp.hovered() { colors.node_hover_bg } else { colors.node_bg };
                    ui.painter().rect_filled(rect, 4.0, bg);
                    let galley = ui.painter().layout_no_wrap(
                        label.clone(),
                        egui::FontId::proportional(13.0),
                        colors.node_text,
                    );
                    let pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5);
                    ui.painter().galley(pos, galley, colors.node_text);
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
        let menu = ContextMenu::new("loom").items(loom_node_context_items(&node.state));
        if let Some(confirmed_id) = menu.show_on(&resp) {
            if let Some(action) = loom_node_action_for_id(confirmed_id, &node.state) {
                event = Some(self.event_for(action, node));
            }
        }
        if resp.has_focus() && ui.input(|i| i.key_pressed(egui::Key::F10) && i.modifiers.shift) {
            crate::context_menu::request_open(ui.ctx(), resp.id, resp.rect.left_bottom());
        }
        event
    }

    fn event_for(&self, action: LoomNodeMenuAction, node: &GraphNode) -> LoomGraphEvent {
        let block_id = node.state.block_id.clone();
        match action {
            LoomNodeMenuAction::Open => LoomGraphEvent::Open { block_id },
            LoomNodeMenuAction::OpenToSide => LoomGraphEvent::OpenToSide { block_id },
            LoomNodeMenuAction::Rename => LoomGraphEvent::Rename {
                block_id,
                current_title: node.title.clone(),
            },
            LoomNodeMenuAction::TogglePin { target } => {
                LoomGraphEvent::SetPinned { block_id, target }
            }
            LoomNodeMenuAction::ToggleFavorite { target } => {
                LoomGraphEvent::SetFavorite { block_id, target }
            }
            LoomNodeMenuAction::CopyBlockId => LoomGraphEvent::CopyBlockId { block_id },
            LoomNodeMenuAction::RevealInPanel => LoomGraphEvent::RevealInPanel { block_id },
        }
    }
}

/// Stable AccessKit author_id for a graph node: `loom_node_{block_id}` (slug-safe).
pub fn loom_node_author_id(block_id: &str) -> String {
    format!("loom_node_{}", crate::project_tree::stable_part(block_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(pinned: bool) -> GraphNode {
        GraphNode::new(
            LoomNodeState {
                block_id: "blk-1".to_owned(),
                pinned,
                favorite: false,
                has_edges: false,
            },
            "My Block",
        )
    }

    #[test]
    fn pin_event_sends_flipped_target() {
        let surface = LoomGraphSurface::new(vec![node(false)]);
        assert_eq!(
            surface.event_for(LoomNodeMenuAction::TogglePin { target: true }, &surface.nodes[0]),
            LoomGraphEvent::SetPinned { block_id: "blk-1".to_owned(), target: true },
        );
    }

    #[test]
    fn rename_event_carries_current_title() {
        let surface = LoomGraphSurface::new(vec![node(false)]);
        assert_eq!(
            surface.event_for(LoomNodeMenuAction::Rename, &surface.nodes[0]),
            LoomGraphEvent::Rename {
                block_id: "blk-1".to_owned(),
                current_title: "My Block".to_owned(),
            },
        );
    }

    #[test]
    fn author_id_slug_safe() {
        let id = loom_node_author_id("blk 1/x");
        assert!(id.starts_with("loom_node_"));
        assert!(id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    }
}
