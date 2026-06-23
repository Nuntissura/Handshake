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

/// One graph node rendered by the surface: its cached state + display title, plus the MT-032
/// everything-is-a-Loom-block tooltip metadata (the node's live backlink count, resolved once when
/// known). The node is ALWAYS a Loom block addressable as `loom://{workspace_id}/{block_id}` — the
/// surface's `workspace_id` supplies the workspace half.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraphNode {
    pub state: LoomNodeState,
    pub title: String,
    /// WP-KERNEL-012 MT-032: the count of inbound backlinks the host resolved for this node (from
    /// `GET /knowledge/documents/{id}/backlinks` — reused MT-015 binding). `None` when unresolved; the
    /// tooltip then omits the count line (honestly absent, not a fabricated `0`).
    pub backlink_count: Option<usize>,
}

impl GraphNode {
    pub fn new(state: LoomNodeState, title: impl Into<String>) -> Self {
        Self {
            state,
            title: title.into(),
            backlink_count: None,
        }
    }

    /// Set the resolved backlink count (builder, used by the host after a backlinks fetch resolves).
    pub fn with_backlink_count(mut self, count: usize) -> Self {
        self.backlink_count = Some(count);
        self
    }

    /// The node's `loom://{workspace_id}/{block_id}` address (MT-032). `None` when either id is empty
    /// (RISK-3: no fabricated URI for an unaddressable node).
    pub fn loom_addr(&self, workspace_id: &str) -> Option<crate::loom_address::LoomBlockAddr> {
        let addr = crate::loom_address::LoomBlockAddr::new(workspace_id, &self.state.block_id);
        addr.is_addressable().then_some(addr)
    }
}

/// Colors for the graph nodes, sourced from the active theme by the host.
#[derive(Debug, Clone, Copy)]
pub struct LoomGraphColors {
    pub node_bg: egui::Color32,
    pub node_hover_bg: egui::Color32,
    pub node_text: egui::Color32,
}

/// The native Loom-graph node surface: a set of nodes, each right-clickable for the MT-021 menu. The
/// `workspace_id` (MT-032) is the workspace half of each node's `loom://` address shown in the tooltip;
/// it defaults empty (a tooltip then shows only the block-id half — still a valid loom address once the
/// host installs the workspace).
#[derive(Debug, Clone, Default)]
pub struct LoomGraphSurface {
    pub nodes: Vec<GraphNode>,
    /// The workspace these nodes live in (MT-032 loom:// address). Empty until the host sets it.
    pub workspace_id: String,
}

impl LoomGraphSurface {
    pub fn new(nodes: Vec<GraphNode>) -> Self {
        Self { nodes, workspace_id: String::new() }
    }

    /// Build a surface for an explicit workspace (MT-032), so each node's tooltip shows the full
    /// `loom://{workspace_id}/{block_id}` address.
    pub fn with_workspace(nodes: Vec<GraphNode>, workspace_id: impl Into<String>) -> Self {
        Self { nodes, workspace_id: workspace_id.into() }
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
                // WP-KERNEL-012 MT-032 (E5 "everything is a Loom block"): the node's loom:// URI +
                // backlink count, exposed BOTH as a hover tooltip (operator-visible) and as the
                // AccessKit description (an out-of-process agent reads the loom address by stable id —
                // HBR-SWARM). An unaddressable node (empty block id) shows no loom line (RISK-3).
                let tooltip = node_tooltip_text(node, &self.workspace_id);
                let resp = if let Some(tip) = &tooltip {
                    let tip = tip.clone();
                    resp.on_hover_ui(move |ui| {
                        for line in tip.lines() {
                            ui.label(line);
                        }
                    })
                } else {
                    resp
                };
                let desc = tooltip.clone();
                ui.ctx().accesskit_node_builder(id, move |node_b| {
                    node_b.set_role(accesskit::Role::TreeItem);
                    node_b.set_author_id(author_id.clone());
                    node_b.set_label(label.clone());
                    if let Some(desc) = &desc {
                        node_b.set_description(desc.replace('\n', "; "));
                    }
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

/// The MT-032 tooltip text for a graph node: its `loom://{workspace_id}/{block_id}` URI plus a backlink
/// count line when the count is resolved. `None` when the node is unaddressable (empty block id —
/// RISK-3: no fabricated `loom://` URI). Newline-separated so the tooltip renders one line per fact and
/// the AccessKit description joins them with "; ".
pub fn node_tooltip_text(node: &GraphNode, workspace_id: &str) -> Option<String> {
    let addr = node.loom_addr(workspace_id)?;
    let mut text = addr.to_uri();
    if let Some(count) = node.backlink_count {
        text.push('\n');
        let suffix = if count == 1 { "" } else { "s" };
        text.push_str(&format!("{count} backlink{suffix}"));
    }
    Some(text)
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

    /// MT-032: the node tooltip shows the node's loom:// URI; with a resolved count it adds a backlink
    /// line. Without a count the line is OMITTED (honestly absent, not a fabricated `0 backlinks`).
    #[test]
    fn tooltip_shows_loom_uri_and_optional_backlink_count() {
        let n = node(false);
        // No count resolved yet -> only the loom:// line.
        assert_eq!(node_tooltip_text(&n, "ws-1").as_deref(), Some("loom://ws-1/blk-1"));
        // A resolved count adds a second line, correctly singular/plural.
        let n1 = node(false).with_backlink_count(1);
        assert_eq!(node_tooltip_text(&n1, "ws-1").as_deref(), Some("loom://ws-1/blk-1\n1 backlink"));
        let n3 = node(false).with_backlink_count(3);
        assert_eq!(node_tooltip_text(&n3, "ws-1").as_deref(), Some("loom://ws-1/blk-1\n3 backlinks"));
        let n0 = node(false).with_backlink_count(0);
        assert_eq!(node_tooltip_text(&n0, "ws-1").as_deref(), Some("loom://ws-1/blk-1\n0 backlinks"));
    }

    /// MT-032 RISK-3: a node with an empty block id has no loom:// tooltip (no fabricated URI).
    #[test]
    fn unaddressable_node_has_no_tooltip() {
        let n = GraphNode::new(
            LoomNodeState { block_id: String::new(), pinned: false, favorite: false, has_edges: false },
            "Orphan",
        );
        assert_eq!(node_tooltip_text(&n, "ws-1"), None);
        // A node WITH a block id but no workspace also has no loom URI yet.
        assert_eq!(node_tooltip_text(&node(false), ""), None);
    }
}
