//! Handshake native GUI library crate.
//! A lib + thin bin split (the bin is src/main.rs) so the GUI shell is unit/integration testable
//! via egui_kittest. (The MT-002 contract said "bin only", but its own integration test needs to
//! reach HandshakeApp, which a bin-only crate cannot expose — so a lib target is required.)

pub mod accessibility;
pub mod app;
// WP-KERNEL-012 MT-033 (E5 — CKC embeds / drag-in): the CKC/Atelier side panel whose item rows are
// egui drag-sources (DragPayload::AtelierRef) for dropping CKC media/characters/moodboards into a native
// note (rich-text hsLink embed atom) or onto the canvas (loom:// block reference). Loads LIVE from the
// existing WP-KERNEL-005 atelier backend via backend_client::AtelierClient (no mocks).
pub mod atelier_side_panel;
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
// WP-KERNEL-012 MT-033 (E5 — route-to-Stage): the LOCAL Stage pane (Pillar 17) that DISPLAYS content
// routed to it (a document, a selection, or a CKC item) via the route-to-stage command on the MT-031
// InteractionBus. The deeper Stage backend interop (capture/embed-back with manifest provenance) is E10.
pub mod stage_pane;
pub mod stash_shelf;
pub mod tab_bar;
pub mod theme;
pub mod top_menu_bar;
// WP-KERNEL-012 MT-035 (E5 — unified undo scope): the ONE session-scoped in-memory undo authority
// (UnifiedUndoScope, PaneUndoRing, CrossPaneUndoRing, UndoAction) every editor pane shares through the
// MT-031 InteractionBus. Local-first per-pane Ctrl+Z, a single cross-pane ring for Ctrl+Shift+Z, caps
// 200/50, NO Serialize (session-scoped — never persisted). NOT CRDT undo (that is MT-038/039).
pub mod undo_stack;
pub mod workspace_settings;
