//! Live AccessKit emission for the native Handshake shell (WP-KERNEL-011 MT-025).
//!
//! ## What this module fixes (the recurring live-emission gap)
//!
//! Earlier MTs proved AccessKit *data* exists but did not always wire it into the **live** egui
//! accessibility tree:
//!
//! - MT-003 (theme toggle) DOES emit live — it calls `Response::widget_info`, which fills a real
//!   node in egui's per-frame `accesskit_state`. Out-of-process UIA / kittest can find it.
//! - MT-002 chrome (the "Handshake" title and the backend status line) and MT-005 panes built
//!   AccessKit nodes only in memory: `PaneRegistry::build_accesskit_node` returns an
//!   `accesskit::Node`, but nothing ever pushed those nodes into the frame's live tree, so a model
//!   driving the app out-of-process could not see panes or chrome by `author_id`.
//!
//! This module closes that gap by emitting nodes through egui's own live path
//! (`Context::accesskit_node_builder`), which creates the node in the current frame's
//! `accesskit_state` AND links it to the correct accessibility parent. The result is visible in:
//!
//! - the kittest AccessKit snapshot (in-process), and
//! - the Windows UIA tree (out-of-process), because eframe pushes the same `TreeUpdate` to the
//!   platform adapter every frame.
//!
//! ## Why `Context::accesskit_node_builder` and not a parallel in-memory builder
//!
//! egui owns the live tree. Building a separate `Vec<(NodeId, Node)>` and trying to merge it would
//! duplicate egui's parent-linkage logic and fight its per-frame id allocation. `accesskit_node_builder`
//! is the supported hook: it returns the *same* `accesskit::Node` egui will serialize into the frame's
//! `TreeUpdate`, so author_id / role / label / actions we set here land in the real tree a model reads.
//!
//! ## Stable ids
//!
//! AccessKit `NodeId`s are derived by egui from an `egui::Id`'s u64. A fixed-value `egui::Id`
//! (`Id::from_high_entropy_bits`) therefore yields a fixed `NodeId` across frames and process
//! restarts — exactly what out-of-process steering needs (RISK-1 / CONTROL-1 in the MT-005 design).
//! Chrome widgets get fixed ids declared here; panes derive their stable id from the registry-owned
//! `PaneRenderContext::egui_id` (which is itself derived from the registry's monotonic NodeId).

mod live;
mod registry;
mod snapshot;

pub use live::{
    emit_chrome_node, emit_interactive_node, emit_pane_node, ChromeWidget, STATUS_BAR_NODE_ID,
    TITLE_BAR_NODE_ID,
};
pub use registry::{
    assert_no_unnamed_interactive, DeclaredIdentity, DECLARED_IDENTITIES, INTERACTIVE_ROLES,
    PALETTE_AUTHOR_IDS, PANE_NODE_ID_BASE, THEME_TOGGLE_AUTHOR_ID, THEME_TOGGLE_NODE_ID,
};
pub use snapshot::{collect_tree_snapshot, AccessNodeSnapshot, AccessTreeSnapshot};
