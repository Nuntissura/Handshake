//! Knowledge-surface graph views (WP-KERNEL-012 cluster E3).
//!
//! Houses the native Obsidian-class Loom graph surfaces. MT-021 delivers the local + global
//! force-directed [`graph_view::LoomGraphView`]; later E3 MTs (folder tree, tags, breadcrumbs, canvas)
//! extend this module. The graph binds the EXISTING PostgreSQL/EventLedger backend through the WP-011
//! [`crate::backend_client::LoomGraphClient`] — no new backend, no Tauri.

pub mod canvas_board;
pub mod folder_tree;
pub mod graph_view;
pub mod sidebar_panel;
pub mod tags_panel;
pub mod wiki_page_panel;

// NOTE: `canvas_board::{ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID}` ("canvas.zoom-in"/"canvas.zoom-out")
// intentionally collide by NAME with `graph_view`'s ("graph.zoom.in"/"graph.zoom.out"), so they are NOT
// re-exported flat here. Consumers/tests import them as `graph::canvas_board::ZOOM_IN_AUTHOR_ID`.
pub use canvas_board::{
    placement_author_id, placement_remove_author_id, CanvasEvent, CanvasPlacementCard, EdgeMode,
    LoomCanvasBoard, VisualEdge, ADD_CARD_AUTHOR_ID, DEFAULT_CARD_H, DEFAULT_CARD_W,
    EDGE_MODE_AUTHOR_ID, GROUP_AUTHOR_ID, PAN_LEFT_AUTHOR_ID, PAN_RIGHT_AUTHOR_ID,
    PLACEMENT_AUTHOR_ID_PREFIX, START_EDGE_AUTHOR_ID, STATUS_AUTHOR_ID, ZOOM_VALUE_AUTHOR_ID,
};

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

pub use tags_panel::{
    hub_add_tag_author_id, hub_member_author_id, hub_title_author_id, tag_chip_color,
    tag_chip_color_index, tag_row_author_id, AddTagCandidate, HubMember, LoomTagHubPanel,
    LoomTagsPanel, TagEntry, TagHubEvent, TagsPanelEvent, HUB_ADD_SEARCH_AUTHOR_ID,
    HUB_ADD_TAG_AUTHOR_ID_PREFIX, HUB_MEMBER_AUTHOR_ID_PREFIX, HUB_TITLE_AUTHOR_ID_PREFIX,
    SEARCH_AUTHOR_ID, TAG_CHIP_PALETTE_LEN, TAG_ROW_AUTHOR_ID_PREFIX,
};

pub use sidebar_panel::{
    backlink_row_author_id, breadcrumb_author_id, favorite_remove_author_id, favorite_row_author_id,
    pin_remove_author_id, pin_row_author_id, section_retry_author_id, truncate_label,
    unlinked_row_author_id, BacklinkRow, BreadcrumbEntry, LoomSidebarPanel, SectionKind, SidebarBlock,
    SidebarEvent, UnlinkedRow, BACKLINK_ROW_AUTHOR_ID_PREFIX, BREADCRUMB_AUTHOR_ID_PREFIX,
    FAVORITE_ROW_AUTHOR_ID_PREFIX, MAX_BREADCRUMBS, PIN_ROW_AUTHOR_ID_PREFIX,
    UNLINKED_ROW_AUTHOR_ID_PREFIX,
};

pub use wiki_page_panel::{
    cancel_author_id, content_author_id, edit_area_author_id, edit_author_id, rebuild_author_id,
    retry_author_id, save_author_id, title_author_id, verdict_is_stale, LoomWikiPagePanel,
    WikiPageEvent, CANCEL_AUTHOR_ID_PREFIX, CONTENT_AUTHOR_ID_PREFIX, CONTENT_DISPLAY_CAP,
    EDIT_AREA_AUTHOR_ID_PREFIX, EDIT_AUTHOR_ID_PREFIX, OVERLAY_INPUT_CAP, SAVE_AUTHOR_ID_PREFIX,
    TITLE_AUTHOR_ID_PREFIX,
};
