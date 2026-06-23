//! Handshake native GUI library crate.
//! A lib + thin bin split (the bin is src/main.rs) so the GUI shell is unit/integration testable
//! via egui_kittest. (The MT-002 contract said "bin only", but its own integration test needs to
//! reach HandshakeApp, which a bin-only crate cannot expose — so a lib target is required.)

pub mod accessibility;
pub mod app;
pub mod backend_client;
pub mod canvas_board;
pub mod code_editor;
pub mod command_palette;
pub mod command_registry;
pub mod context_menu;
pub mod context_menu_surfaces;
pub mod debug_console;
pub mod drawer;
pub mod error;
pub mod graph;
pub mod loom_graph;
pub mod source_control;
pub mod event_bus;
pub mod find_in_files;
pub mod installer;
pub mod interop;
pub mod layout_persistence;
pub mod left_rail;
// WP-KERNEL-012 MT-032 (E5): the "everything is a Loom block" addressing layer (LoomBlockAddr,
// ContentHash, loom://, LoomBlockResolver). The MT contract also named a `backlink_panel` module, but
// the KERNEL_BUILDER anti-overlap gate established the backlinks panel ALREADY EXISTS
// (rich_editor/wikilinks/backlinks_panel.rs, MT-015) — MT-032 REUSES it rather than minting a third
// duplicate, so no `backlink_panel` module is declared here.
pub mod loom_address;
pub mod loom_search_v2;
pub mod mcp;
pub mod module_switcher;
pub mod pane_header;
pub mod pane_registry;
pub mod popout_window;
pub mod project_tabs;
pub mod project_tree;
pub mod quick_links;
pub mod quiet_mode;
pub mod quick_switcher;
pub mod rails;
pub mod rich_editor;
pub mod search_rail;
pub mod settings_dialog;
pub mod split_layout;
pub mod stash_shelf;
pub mod tab_bar;
pub mod theme;
pub mod top_menu_bar;
pub mod workspace_settings;
