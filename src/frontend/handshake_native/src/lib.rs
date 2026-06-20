//! Handshake native GUI library crate.
//! A lib + thin bin split (the bin is src/main.rs) so the GUI shell is unit/integration testable
//! via egui_kittest. (The MT-002 contract said "bin only", but its own integration test needs to
//! reach HandshakeApp, which a bin-only crate cannot expose — so a lib target is required.)

pub mod accessibility;
pub mod app;
pub mod backend_client;
pub mod error;
pub mod layout_persistence;
pub mod module_switcher;
pub mod pane_registry;
pub mod popout_window;
pub mod project_tabs;
pub mod rails;
pub mod split_layout;
pub mod tab_bar;
pub mod theme;
