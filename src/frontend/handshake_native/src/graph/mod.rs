//! Knowledge-surface graph views (WP-KERNEL-012 cluster E3).
//!
//! Houses the native Obsidian-class Loom graph surfaces. MT-021 delivers the local + global
//! force-directed [`graph_view::LoomGraphView`]; later E3 MTs (folder tree, tags, breadcrumbs, canvas)
//! extend this module. The graph binds the EXISTING PostgreSQL/EventLedger backend through the WP-011
//! [`crate::backend_client::LoomGraphClient`] — no new backend, no Tauri.

pub mod folder_tree;
pub mod graph_view;

pub use graph_view::{
    content_type_color, node_author_id, GraphEdge, GraphEvent, GraphMode, GraphNode, LoomGraphView,
    MAX_LAYOUT_ITERS, MODE_GLOBAL_AUTHOR_ID, MODE_LOCAL_AUTHOR_ID, NODE_AUTHOR_ID_PREFIX, NODE_CAP,
    RELAYOUT_AUTHOR_ID, ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID,
};

pub use folder_tree::{
    build_tree, color_author_id, color_to_hex, parse_hex_color, FolderNode, FolderRow,
    FolderTreeEvent, LeafBlock, LoomFolderTree, COLOR_AUTHOR_ID_PREFIX,
    NODE_AUTHOR_ID_PREFIX as FOLDER_TREE_NODE_AUTHOR_ID_PREFIX, RETRY_AUTHOR_ID,
};
