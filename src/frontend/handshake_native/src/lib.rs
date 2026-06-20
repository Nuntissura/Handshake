//! Handshake native GUI library crate.
//! A lib + thin bin split (the bin is src/main.rs) so the GUI shell is unit/integration testable
//! via egui_kittest. (The MT-002 contract said "bin only", but its own integration test needs to
//! reach HandshakeApp, which a bin-only crate cannot expose — so a lib target is required.)

pub mod accessibility;
pub mod app;
pub mod backend_client;
pub mod command_palette;
pub mod command_registry;
pub mod context_menu;
pub mod error;
pub mod event_bus;
pub mod layout_persistence;
pub mod left_rail;
pub mod module_switcher;
pub mod pane_header;
pub mod pane_registry;
pub mod popout_window;
pub mod project_tabs;
pub mod project_tree;
pub mod quick_links;
pub mod quick_switcher;
pub mod rails;
pub mod settings_dialog;
pub mod split_layout;
pub mod tab_bar;
pub mod theme;
pub mod top_menu_bar;
pub mod workspace_settings;
