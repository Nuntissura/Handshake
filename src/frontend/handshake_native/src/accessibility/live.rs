//! Live AccessKit node emission helpers.
//!
//! Each helper takes an `egui::Id` already allocated for a real widget (so the node attaches to the
//! correct accessibility parent) and enriches the corresponding live node with a stable `author_id`,
//! a semantic `Role`, and a human/model-readable `label`. Setting these through
//! `Context::accesskit_node_builder` writes into the frame's live `accesskit_state`, so the values
//! appear in the kittest snapshot and the out-of-process UIA tree.

use egui::accesskit;

/// Fixed AccessKit/egui id for the top title bar's "Handshake" identity widget. A low fixed value
/// is collision-safe for a single hand-assigned widget (entropy only affects egui's `IdMap`
/// distribution; one fixed widget cannot self-collide). Kept distinct from the theme toggle (id 10
/// in `app.rs`) and from the pane id base (100+ in `pane_registry.rs`).
pub const TITLE_BAR_NODE_ID: u64 = 20;

/// Fixed AccessKit/egui id for the bottom status bar's backend-health widget.
pub const STATUS_BAR_NODE_ID: u64 = 21;

/// The chrome widgets that carry a stable AccessKit identity. Each maps to a fixed `egui::Id` and a
/// stable kebab-case `author_id`, so a model can address shell chrome the same way it addresses
/// panes — by `author_id` — without depending on display text that may localize or change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeWidget {
    /// The top-bar "Handshake" identity / title region.
    TitleBar,
    /// The bottom status bar showing live backend health.
    StatusBar,
}

impl ChromeWidget {
    /// Fixed u64 backing both the `egui::Id` and (via egui) the AccessKit `NodeId`.
    pub fn node_id(self) -> u64 {
        match self {
            ChromeWidget::TitleBar => TITLE_BAR_NODE_ID,
            ChromeWidget::StatusBar => STATUS_BAR_NODE_ID,
        }
    }

    /// Stable, model-meaningful match key (kebab-case, dot-namespaced under `shell.`). This is the
    /// out-of-process address a model uses, mirroring the pane `author_id` convention.
    pub fn author_id(self) -> &'static str {
        match self {
            ChromeWidget::TitleBar => "shell.chrome.title-bar",
            ChromeWidget::StatusBar => "shell.chrome.status-bar",
        }
    }

    /// Semantic AccessKit role for the chrome region.
    pub fn role(self) -> accesskit::Role {
        match self {
            ChromeWidget::TitleBar => accesskit::Role::TitleBar,
            ChromeWidget::StatusBar => accesskit::Role::Status,
        }
    }

    /// The fixed `egui::Id` for this chrome widget. `from_high_entropy_bits` is the same mechanism
    /// the MT-003 theme toggle uses to pin a stable `NodeId`.
    ///
    /// # Safety
    /// `from_high_entropy_bits` is `unsafe` because egui assumes the value is well-distributed for
    /// its `IdMap`. A single hand-assigned, never-reused fixed id is safe: it cannot self-collide,
    /// and the values here (20, 21) are deliberately disjoint from the toggle (10) and pane ids
    /// (100+).
    pub fn egui_id(self) -> egui::Id {
        unsafe { egui::Id::from_high_entropy_bits(self.node_id()) }
    }
}

/// Emit a live AccessKit node for a shell chrome widget.
///
/// `widget_id` is the `egui::Id` of the real widget that was already allocated this frame (e.g. the
/// id returned by `ui.interact(rect, ChromeWidget::TitleBar.egui_id(), ...)`). Passing the widget's
/// real id ensures the node attaches under the correct panel in the accessibility tree rather than
/// floating at the root. `label` is the current display text (e.g. "Handshake" or the backend
/// status string) so the node carries both the stable `author_id` and the live human-readable text.
///
/// No-op when the `accesskit` feature is disabled on egui or AccessKit is not active this frame
/// (`accesskit_node_builder` returns `None`); chrome still renders, it simply carries no a11y node,
/// matching egui's own graceful-degradation contract.
pub fn emit_chrome_node(ctx: &egui::Context, chrome: ChromeWidget, widget_id: egui::Id, label: &str) {
    ctx.accesskit_node_builder(widget_id, |node| {
        node.set_role(chrome.role());
        node.set_author_id(chrome.author_id().to_owned());
        node.set_label(label.to_owned());
    });
}

/// Emit a live AccessKit node for a work-surface pane.
///
/// This is the live counterpart to `PaneRegistry::build_accesskit_node` (which only built an
/// in-memory node). It consumes the MT-005 `PaneRenderContext::egui_id` as the widget id and the
/// `PaneFactory::accesskit_role()` as the role, and uses the pane's kebab-case id as the
/// `author_id` so an out-of-process model addresses the pane by the same stable key the registry
/// records.
///
/// - `pane_egui_id`: `PaneRenderContext::egui_id` (stable, derived from the registry NodeId).
/// - `pane_author_id`: the kebab-case `pane_id` string (e.g. `"pane-a"`).
/// - `role`: `PaneFactory::accesskit_role()` for this pane's surface.
/// - `label`: `PaneType::label()` (human/model-readable surface name).
pub fn emit_pane_node(
    ctx: &egui::Context,
    pane_egui_id: egui::Id,
    pane_author_id: &str,
    role: accesskit::Role,
    label: &str,
) {
    ctx.accesskit_node_builder(pane_egui_id, |node| {
        node.set_role(role);
        node.set_author_id(pane_author_id.to_owned());
        node.set_label(label.to_owned());
    });
}
