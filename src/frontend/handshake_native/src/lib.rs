//! Handshake native GUI library crate.
//! A lib + thin bin split (the bin is src/main.rs) so the GUI shell is unit/integration testable
//! via egui_kittest. (The MT-002 contract said "bin only", but its own integration test needs to
//! reach HandshakeApp, which a bin-only crate cannot expose — so a lib target is required.)

pub mod accessibility;
pub mod app;
// WP-KERNEL-012 MT-037 (E6 — backend reuse wiring): the typed native clients for the EXISTING
// handshake_core HTTP surfaces. MT-037 adds backend::knowledge_documents — the consolidated typed
// client for the full /knowledge/documents/* route family (create/import/load/draft/save/blocks/
// history/projection/embeds/backlinks/rename/move/batch), reusing the WP-011 backend_client identity
// headers + base URL rather than forking a second HTTP stack.
pub mod backend;
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
// WP-KERNEL-012 MT-079 (E11 host-mount): the session-threaded editor pane factories that mount the REAL
// native code + rich-text editors into the running HandshakeApp shell (replacing the PlaceholderPaneFactory
// for the editor PaneTypes), thread per-pane session context (runtime/workspace/embed/wikilink) through the
// established shared-cell pattern, wire the shell command sender into the code pane, and drain the rich
// editor's pending_events to the shell's nav-bus routing. The CORE mount (AC-079-1..5); the fuller
// canvas/graph/side-pane mounts stay typed carries.
pub mod editor_pane_factories;
pub mod error;
// WP-KERNEL-012 MT-036 (E5 — one event ledger across surfaces): the single NativeEditorEventEmitter that
// turns a native editor action into a typed NativeEditorEvent and ships it to the EXISTING handshake_core
// Flight Recorder ledger (Semaphore-bounded off-frame spawn + cap-20 in-memory error ring). LIVE-wired at
// the rich-text save, rich-pane undo, and route-to-stage call sites; the code-edit + canvas live emits are
// honestly DEFERRED to E11/MT-069. The full native→ledger round-trip is a TYPED BACKEND BLOCKER (the real
// backend has no ingestion endpoint accepting a native-editor event — see the module doc).
pub mod event_emitter;
// WP-KERNEL-012 MT-036 (E5 — flight recorder pane): the native port of FlightRecorderView.tsx listing the
// native editor events the ledger holds (HBR-VIS/HBR-SWARM). No perpetual spinner; theme tokens only;
// flight-recorder-pane(Region) + fr-event-{id}(ListItem) AccessKit nodes.
pub mod flight_recorder_pane;
pub mod graph;
pub mod loom_graph;
pub mod source_control;
pub mod event_bus;
// WP-KERNEL-012 MT-063 (E9 — FEMS interop): the editors' READ-ONLY consumer of the Pillar 12 FEMS
// retrieval capsule (MemoryPack). `fems::memory_client` is the typed read client + deserialized
// MemoryPack model (3 kinds, provenance-first source, <=24 items hard-capped client-side, <=500 token
// advisory budget) that reuses the WP-011 `backend_client` shared reqwest pool; `fems::relevant_memory_panel`
// is the side panel that renders the capsule provenance-first. The FEMS read route is ABSENT in the
// current handshake_core build, so `fetch_pack` returns the typed blocker `EndpointMissing` and the panel
// renders the empty-state banner (the designed primary path — no backend add).
pub mod fems;
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
// WP-KERNEL-012 MT-036 (E5 — designed extension seams): the DESIGN-ONLY EditorSurface trait +
// EditorSurfaceRegistry by which FUTURE surfaces (image editor, spreadsheet, engine) attach to the shared
// selection/event-ledger/undo substrate WITHOUT touching the existing emitter. Compiles + is unit-proven
// object-safe; #[allow(dead_code)] — no production code calls it at runtime (the contract's explicit seam).
pub mod surface_extension_seam;
pub mod tab_bar;
pub mod theme;
pub mod top_menu_bar;
// WP-KERNEL-012 MT-035 (E5 — unified undo scope): the ONE session-scoped in-memory undo authority
// (UnifiedUndoScope, PaneUndoRing, CrossPaneUndoRing, UndoAction) every editor pane shares through the
// MT-031 InteractionBus. Local-first per-pane Ctrl+Z, a single cross-pane ring for Ctrl+Shift+Z, caps
// 200/50, NO Serialize (session-scoped — never persisted). NOT CRDT undo (that is MT-038/039).
pub mod undo_stack;
pub mod workspace_settings;
