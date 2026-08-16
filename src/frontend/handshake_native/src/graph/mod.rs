//! Knowledge-surface graph views (WP-KERNEL-012 cluster E3).
//!
//! Houses the native Obsidian-class Loom graph surfaces. MT-021 delivers the local + global
//! force-directed [`graph_view::LoomGraphView`]; later E3 MTs (folder tree, tags, breadcrumbs, canvas)
//! extend this module. The graph binds the EXISTING SurrealDB/EventLedger backend through the WP-011
//! [`crate::backend_client::LoomGraphClient`] — no new backend, no Tauri.

pub mod block_collection_view;
pub mod canvas_board;
// WP-KERNEL-012 MT-061 (E3): Obsidian-Canvas section/group FRAMES for the MT-026 canvas. Derives a
// titled rounded-rectangle container per distinct placement `group_id` (drawn behind the cards) and owns
// the deterministic which_section(drop_pos) hit-testing the canvas uses for drag-drop section assignment.
pub mod canvas_sections;
// WP-KERNEL-012 MT-067 (E10 — Calendar/Pillar 2 interop): the daily journal panel host surface that melts
// the MT-019 daily-note system together with the Calendar. It REUSES the MT-019 date nav (no second
// date-picker) and the `crate::interop::CalendarInteropService` reads, renders a clickable CalendarEvent
// chip (bus-only focus, no calendar-pane import) and a read-only "Edited during this block" activity strip,
// and shows the typed-blocker empty-states for the absent `/calendar/` routes. The loom surface is `graph/`
// (the contract's `loom/` path is fictional — VERIFIED), so the panel lives here alongside the graph views.
pub mod daily_journal_panel;
pub mod folder_tree;
// WP-KERNEL-012 MT-060 (E3): the Obsidian-class graph control panel (search filter / tag+folder groups /
// link-depth slider / orphan + size-by-degree toggles) rendered alongside the MT-021 graph canvas. Pure
// filter/group/sizing fns are unit-testable; the panel re-fires ONLY the existing graph-search endpoint
// on a depth change (Local mode), all other controls are client-side over the loaded vecs.
pub mod graph_controls;
pub mod graph_view;
pub mod sidebar_panel;
pub mod tags_panel;
pub mod wiki_page_panel;

// NOTE: `canvas_board::{ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID}` ("canvas.zoom-in"/"canvas.zoom-out")
// intentionally collide by NAME with `graph_view`'s ("graph.zoom.in"/"graph.zoom.out"), so they are NOT
// re-exported flat here. Consumers/tests import them as `graph::canvas_board::ZOOM_IN_AUTHOR_ID`.
pub use canvas_board::{
    placement_author_id, placement_remove_author_id, CanvasDragPayload, CanvasEvent,
    CanvasPlacementCard, EdgeMode, LoomCanvasBoard, VisualEdge, ADD_CARD_AUTHOR_ID, DEFAULT_CARD_H,
    DEFAULT_CARD_W, EDGE_MODE_AUTHOR_ID, GROUP_AUTHOR_ID, PAN_LEFT_AUTHOR_ID, PAN_RIGHT_AUTHOR_ID,
    PLACEMENT_AUTHOR_ID_PREFIX, PLACE_BLOCK_AUTHOR_ID, PLACE_BLOCK_INPUT_AUTHOR_ID,
    START_EDGE_AUTHOR_ID, STATUS_AUTHOR_ID, ZOOM_VALUE_AUTHOR_ID,
};

// WP-KERNEL-012 MT-061 (E3): the canvas section/group frame layer + hit-testing + AccessKit helper,
// re-exported flat for the host pane + the proof tests.
pub use canvas_sections::{
    section_author_id, SectionFrame, SectionLayer, SECTION_AUTHOR_ID_PREFIX,
};

pub use graph_view::{
    content_type_color, node_author_id, GraphEdge, GraphEvent, GraphMode, GraphNode, LoomGraphView,
    MAX_LAYOUT_ITERS, MODE_GLOBAL_AUTHOR_ID, MODE_LOCAL_AUTHOR_ID, NODE_AUTHOR_ID_PREFIX, NODE_CAP,
    RELAYOUT_AUTHOR_ID, ZOOM_IN_AUTHOR_ID, ZOOM_OUT_AUTHOR_ID,
};

// WP-KERNEL-012 MT-060 (E3): the graph control panel state + pure filter/group/sizing fns + AccessKit
// author_ids, re-exported flat for the host pane + the proof tests.
pub use graph_controls::{
    assign_group_color, compute_visibility, group_author_id, node_degree, node_radius,
    GraphControls, GraphControlsEvent, GraphGroup, GroupKind, NodeVisibility, DEPTH_AUTHOR_ID,
    DIM_ALPHA, GROUP_AUTHOR_ID_PREFIX, ORPHAN_AUTHOR_ID,
    SEARCH_AUTHOR_ID as GRAPH_FILTER_SEARCH_AUTHOR_ID, SIZE_DEGREE_AUTHOR_ID, TOGGLE_AUTHOR_ID,
};

// WP-KERNEL-012 MT-067 (E10): the daily journal panel + its state/event types + AccessKit author_ids,
// re-exported flat for the host pane + the proof tests.
pub use daily_journal_panel::{
    activity_item_author_id, collect_edited_doc_ids, ActivityCorrelation, DailyJournalEvent,
    DailyJournalPanel, DailyJournalState, DAILY_JOURNAL_ACTIVITY_ITEM_AUTHOR_ID_PREFIX,
    DAILY_JOURNAL_ACTIVITY_STRIP_AUTHOR_ID, DAILY_JOURNAL_CALENDAR_EVENT_CHIP_AUTHOR_ID,
    DAILY_JOURNAL_DATE_HEADER_AUTHOR_ID, DAILY_JOURNAL_PANEL_AUTHOR_ID,
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
    backlink_row_author_id, breadcrumb_author_id, favorite_remove_author_id,
    favorite_row_author_id, pin_remove_author_id, pin_row_author_id, section_retry_author_id,
    truncate_label, unlinked_row_author_id, BacklinkRow, BreadcrumbEntry, LoomSidebarPanel,
    SectionKind, SidebarBlock, SidebarEvent, UnlinkedRow, BACKLINK_ROW_AUTHOR_ID_PREFIX,
    BREADCRUMB_AUTHOR_ID_PREFIX, FAVORITE_ROW_AUTHOR_ID_PREFIX, MAX_BREADCRUMBS,
    PIN_ROW_AUTHOR_ID_PREFIX, UNLINKED_ROW_AUTHOR_ID_PREFIX,
};

pub use wiki_page_panel::{
    cancel_author_id, content_author_id, edit_area_author_id, edit_author_id, rebuild_author_id,
    retry_author_id, save_author_id, title_author_id, verdict_is_stale, LoomWikiPagePanel,
    WikiPageEvent, CANCEL_AUTHOR_ID_PREFIX, CONTENT_AUTHOR_ID_PREFIX, CONTENT_DISPLAY_CAP,
    EDIT_AREA_AUTHOR_ID_PREFIX, EDIT_AUTHOR_ID_PREFIX, OVERLAY_INPUT_CAP, SAVE_AUTHOR_ID_PREFIX,
    TITLE_AUTHOR_ID_PREFIX,
};

// MT-027 BlockCollectionViews (table / Kanban / calendar saved-view host). The author-id helpers and
// host/sub-view types re-export flat for the host pane + the proof tests. The `bcv.*` author-id
// namespace is unique, so `STATUS_AUTHOR_ID` is re-exported under an aliased name to avoid colliding
// with `canvas_board::STATUS_AUTHOR_ID` (which is NOT re-exported flat) at the module path level.
pub use block_collection_view::STATUS_AUTHOR_ID as BCV_STATUS_AUTHOR_ID;
pub use block_collection_view::{
    bucket_key, calendar_day_author_id, calendar_entry_author_id, card_move_tags, flip_direction,
    is_iso_date, kanban_card_author_id, kanban_lane_author_id, table_row_author_id,
    table_sort_author_id, BlockCollectionView, BlockViewDefinition, BlockViewEvent, BlockViewField,
    BlockViewKind, BlockViewLane, BlockViewQuery, BlockViewResults, BlockViewSort,
    BlockViewSortDirection, CalendarSubView, KanbanDragState, KanbanSubView, LoomBlockRow,
    TableSubView, BLOCK_VIEW_UNTAGGED_LANE, CALENDAR_DATE_FROM_AUTHOR_ID,
    CALENDAR_DATE_TO_AUTHOR_ID, CALENDAR_DAY_AUTHOR_ID_PREFIX, CALENDAR_ENTRY_AUTHOR_ID_PREFIX,
    KANBAN_CARD_AUTHOR_ID_PREFIX, KANBAN_DRAG_MIME, KANBAN_LANE_AUTHOR_ID_PREFIX,
    KIND_CALENDAR_AUTHOR_ID, KIND_KANBAN_AUTHOR_ID, KIND_TABLE_AUTHOR_ID, NEW_VIEW_AUTHOR_ID,
    NEW_VIEW_CONFIRM_AUTHOR_ID, NEW_VIEW_KIND_TABLE_AUTHOR_ID, NEW_VIEW_TITLE_AUTHOR_ID,
    TABLE_ROW_AUTHOR_ID_PREFIX, TABLE_SORT_AUTHOR_ID_PREFIX,
};

/// MT-031 (E5 melt-together): the graph + canvas surfaces' thin adapter into the shared
/// [`crate::interop::InteractionBus`]. A graph node / canvas card selection feeds the ONE
/// [`crate::interop::interaction_bus::SharedSelection`], and a node copy goes to the ONE shared
/// clipboard as a `loom://{block_id}` reference (the contract's "copy node ref as loom:// URI") rather
/// than ad-hoc per-pane clipboard state (AC-7). These are the concrete `bus.register_command` +
/// `bus.clipboard_write` call sites for the graph + canvas surfaces. `loom_graph.rs`'s node identity is
/// the source of the `node_id` / `block_id` (reuse, not a new node model).
pub mod interop_adapter {
    use crate::interop::adapters::{copy_selection_to_clipboard, register_standard_commands};
    use crate::interop::interaction_bus::{EditorSurfaceKind, InteractionBus, SharedSelection};
    use crate::pane_registry::PaneId;
    use crate::rich_editor::properties::metadata_client::ClipboardSink;

    /// Register the graph surface's melt-together command set into the shared bus (AC-4). Called once
    /// when the graph pane mounts.
    pub fn register_graph(bus: &mut InteractionBus) {
        register_standard_commands(bus, EditorSurfaceKind::Graph);
    }

    /// Register the canvas surface's melt-together command set into the shared bus (AC-4). Called once
    /// when the canvas (Loom canvas board, MT-026) pane mounts.
    pub fn register_canvas(bus: &mut InteractionBus) {
        register_standard_commands(bus, EditorSurfaceKind::Canvas);
    }

    /// Build a [`SharedSelection::NodeRef`] for a selected graph node (the graph pane publishes this to
    /// the bus when the selected node changes). `block_id` comes from `loom_graph.rs` node identity.
    pub fn graph_node_selection(pane_id: PaneId, block_id: impl Into<String>) -> SharedSelection {
        SharedSelection::NodeRef {
            pane_id,
            surface: EditorSurfaceKind::Graph,
            node_id: block_id.into(),
        }
    }

    /// Build a [`SharedSelection::NodeRef`] for a selected canvas placement's referenced block (the
    /// canvas pane publishes this when its selection changes). `placed_block_id` is the canvas card's
    /// referenced Loom block.
    pub fn canvas_node_selection(
        pane_id: PaneId,
        placed_block_id: impl Into<String>,
    ) -> SharedSelection {
        SharedSelection::NodeRef {
            pane_id,
            surface: EditorSurfaceKind::Canvas,
            node_id: placed_block_id.into(),
        }
    }

    /// Copy a graph/canvas node-ref selection to the shared clipboard as a `loom://{block_id}` reference
    /// through the bus (the Ctrl+C / "copy block id" path). Returns `true` when a node ref was copied.
    /// OS write goes through the mockable [`ClipboardSink`] (headless-safe — MT-017 precedent), and the
    /// bus caches the rich `LoomBlockRef` for a cross-pane Paste.
    pub fn copy_node_to_bus(
        bus: &mut InteractionBus,
        selection: &SharedSelection,
        sink: &dyn ClipboardSink,
    ) -> bool {
        copy_selection_to_clipboard(bus, selection, sink)
    }

    use crate::backend_client::{
        CanvasBoardClient, CanvasCreateReceiptError, CreatedCanvasPlacement,
    };
    use crate::undo_stack::{AsyncUndoDirection, UndoAction, UndoAsyncFn, UndoFn, UndoResult};
    use std::sync::{Arc, Mutex, Weak};

    #[derive(Clone, Debug)]
    pub(super) struct CanvasPlacementUndoState {
        current_placement_id: Arc<Mutex<String>>,
    }

    impl CanvasPlacementUndoState {
        pub(super) fn new(placement_id: String) -> Self {
            Self {
                current_placement_id: Arc::new(Mutex::new(placement_id)),
            }
        }

        pub(super) fn current_placement_id(&self) -> String {
            self.current_placement_id
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone()
        }

        pub(super) fn accept_redo_created_placement(&self, placement: CreatedCanvasPlacement) {
            *self
                .current_placement_id
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = placement.placement_id;
        }
    }

    /// Establish the redo replacement identity before making the completion observable to the shell.
    /// The shell treats a successful completion as permission to put the action back on the undo ring;
    /// publishing first would let a rapid next undo read the deleted pre-redo placement id.
    pub(super) fn commit_redo_identity_then_publish(
        state: &CanvasPlacementUndoState,
        created: CreatedCanvasPlacement,
        publish: impl FnOnce(),
    ) {
        state.accept_redo_created_placement(created);
        publish();
    }

    pub(super) fn fail_closed_redo_receipt(
        error: CanvasCreateReceiptError,
    ) -> Result<CreatedCanvasPlacement, String> {
        Err(format!(
            "{error}; redo failed closed because no durable operation token proves placement ownership"
        ))
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CanvasBoardReloadIntent {
        pub workspace_id: String,
        pub canvas_block_id: String,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct CanvasCompensationCompletion {
        pub action_id: String,
        pub direction: AsyncUndoDirection,
        pub reload: CanvasBoardReloadIntent,
        pub result: Result<(), String>,
    }

    pub type CanvasCompensationCell =
        Arc<Mutex<std::collections::VecDeque<CanvasCompensationCompletion>>>;

    fn publish_canvas_compensation(
        cell: &CanvasCompensationCell,
        completion: CanvasCompensationCompletion,
    ) {
        cell.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push_back(completion);
    }

    /// MT-035 (E5 unified undo): record a LOCAL graph-edit undo action for `pane_id` (POLICY-1) — a node
    /// MOVE or a TAG change. `undo_apply`/`redo_apply` mutate the graph pane's in-memory state (e.g.
    /// restore the node's previous position / tag set); both capture a `Weak` back-ref to that state
    /// (RISK-3 / MC-3) and report a benign [`UndoResult::pane_dropped`] when the pane closed. Graph node
    /// moves are LOCAL (in-pane layout), so they go on the pane ring — NOT the cross-pane ring (which is
    /// for backend-touching atomic actions). `loom_graph.rs` node identity is the source of the node id
    /// (reuse, not a new node model).
    pub fn push_graph_undo<S>(
        bus: &mut InteractionBus,
        pane_id: PaneId,
        graph_state: &Arc<std::sync::Mutex<S>>,
        undo_apply: Arc<dyn Fn(&mut S) + Send + Sync>,
        redo_apply: Arc<dyn Fn(&mut S) + Send + Sync>,
        description: impl Into<String>,
    ) where
        S: Send + 'static,
    {
        let weak: Weak<std::sync::Mutex<S>> = Arc::downgrade(graph_state);
        let undo_weak = weak.clone();
        let undo_fn: UndoFn = Arc::new(move || match undo_weak.upgrade() {
            Some(state) => {
                undo_apply(&mut state.lock().unwrap_or_else(|e| e.into_inner()));
                UndoResult::ok()
            }
            None => UndoResult::pane_dropped(),
        });
        let redo_fn: UndoFn = Arc::new(move || match weak.upgrade() {
            Some(state) => {
                redo_apply(&mut state.lock().unwrap_or_else(|e| e.into_inner()));
                UndoResult::ok()
            }
            None => UndoResult::pane_dropped(),
        });
        bus.push_undo_local(pane_id, UndoAction::sync(description, undo_fn, redo_fn));
    }

    /// MT-035 (E5 unified undo) — POLICY-4 CANVAS COMPENSATING undo. A canvas node CREATION
    /// (`POST .../placements`) calls the backend immediately, so it is NOT in-memory undoable; its undo
    /// is a COMPENSATING backend call: `DELETE .../canvas-placements/{placement_id}` (the MT-026 verified
    /// route — NOT the contract's stale `PUT /canvas/{id}/graph`). The created placement's id is captured
    /// AT ACTION-CREATE TIME (RISK-2 / MC-2) so an intermediate canvas edit before the undo fires cannot
    /// clobber it — undo removes exactly the placement this action created. Redo re-places the SAME block
    /// at the SAME geometry (`POST .../placements`). This goes on the CROSS-PANE ring (POLICY-2): a canvas
    /// placement is a backend-touching atomic action undone by Ctrl+Shift+Z.
    ///
    /// `client` is the shared [`CanvasBoardClient`] (its own `reqwest::Client` + runtime handle clone),
    /// so the compensating call runs through the SAME verified transport the live board uses (no new
    /// backend, reuse-only). The async closures send the prebuilt `RequestSpec` and await the result; the
    /// bus dispatches them onto the tokio runtime off the egui frame thread (HBR-QUIET).
    #[allow(clippy::too_many_arguments)] // ws/canvas/placement/block/x/y/w/h — the verified placement shape.
    pub fn push_canvas_placement_undo(
        bus: &mut InteractionBus,
        client: Arc<CanvasBoardClient>,
        workspace_id: String,
        canvas_block_id: String,
        placement_id: String,
        placed_block_id: String,
        geometry: (f64, f64, f64, f64),
        completion_cell: CanvasCompensationCell,
        repaint_ctx: egui::Context,
        description: impl Into<String>,
    ) {
        let (x, y, w, h) = geometry;
        let action_id = uuid::Uuid::new_v4().to_string();
        let reload = CanvasBoardReloadIntent {
            workspace_id: workspace_id.clone(),
            canvas_block_id: canvas_block_id.clone(),
        };
        let placement_state = CanvasPlacementUndoState::new(placement_id);
        // UNDO = compensating DELETE of the latest created placement id. The first undo targets the
        // original backend-minted id; redo updates this shared state to the replacement id so a second
        // undo removes the redone placement rather than retrying the stale original id.
        let undo_client = client.clone();
        let undo_ws = workspace_id.clone();
        let undo_canvas = canvas_block_id.clone();
        let undo_state = placement_state.clone();
        let undo_action_id = action_id.clone();
        let undo_reload = reload.clone();
        let undo_completion = Arc::clone(&completion_cell);
        let undo_repaint = repaint_ctx.clone();
        let undo_async_fn: UndoAsyncFn = Arc::new(move || {
            let client = undo_client.clone();
            let ws = undo_ws.clone();
            let canvas = undo_canvas.clone();
            let placement = undo_state.current_placement_id();
            let action_id = undo_action_id.clone();
            let reload = undo_reload.clone();
            let completion = Arc::clone(&undo_completion);
            let repaint = undo_repaint.clone();
            Box::pin(async move {
                let spec = client.remove_placement_request(&ws, &placement);
                let result = match send_canvas_compensation(&client, spec).await {
                    Ok(()) => Ok(()),
                    Err(receipt_error) => match client.fetch_board_now(&ws, &canvas).await {
                        Ok(board)
                            if !board
                                .placements
                                .iter()
                                .any(|row| row.placement_id == placement) =>
                        {
                            // The DELETE committed but its response was lost/malformed. Canonical
                            // absence is the authoritative success receipt.
                            Ok(())
                        }
                        Ok(_) => Err(format!(
                            "{receipt_error}; canonical board still contains placement {placement}"
                        )),
                        Err(reconcile_error) => Err(format!(
                            "{receipt_error}; canonical delete reconciliation failed: {reconcile_error}"
                        )),
                    },
                };
                publish_canvas_compensation(
                    &completion,
                    CanvasCompensationCompletion {
                        action_id,
                        direction: AsyncUndoDirection::Undo,
                        reload,
                        result: result.clone(),
                    },
                );
                repaint.request_repaint();
                match result {
                    Ok(()) => UndoResult::ok(),
                    Err(e) => {
                        UndoResult::err(format!("canvas undo (remove placement) failed: {e}"))
                    }
                }
            })
        });
        // REDO = re-place the SAME block at the SAME geometry (POST .../placements).
        let redo_client = client.clone();
        let redo_ws = workspace_id.clone();
        let redo_canvas = canvas_block_id.clone();
        let redo_block = placed_block_id.clone();
        let redo_state = placement_state.clone();
        let redo_action_id = action_id.clone();
        let redo_reload = reload;
        let redo_completion = Arc::clone(&completion_cell);
        let redo_repaint = repaint_ctx;
        let redo_async_fn: UndoAsyncFn = Arc::new(move || {
            let client = redo_client.clone();
            let ws = redo_ws.clone();
            let canvas = redo_canvas.clone();
            let block = redo_block.clone();
            let state = redo_state.clone();
            let action_id = redo_action_id.clone();
            let reload = redo_reload.clone();
            let completion = Arc::clone(&redo_completion);
            let repaint = redo_repaint.clone();
            Box::pin(async move {
                let spec = client.place_block_request(&ws, &canvas, &block, x, y, w, h);
                let result = match send_canvas_created_compensation(&client, spec).await {
                    Ok(created) => Ok(created),
                    Err(receipt_error) => fail_closed_redo_receipt(receipt_error),
                };
                match result {
                    Ok(created) => {
                        commit_redo_identity_then_publish(&state, created, || {
                            publish_canvas_compensation(
                                &completion,
                                CanvasCompensationCompletion {
                                    action_id,
                                    direction: AsyncUndoDirection::Redo,
                                    reload,
                                    result: Ok(()),
                                },
                            );
                            repaint.request_repaint();
                        });
                        UndoResult::ok()
                    }
                    Err(e) => {
                        publish_canvas_compensation(
                            &completion,
                            CanvasCompensationCompletion {
                                action_id,
                                direction: AsyncUndoDirection::Redo,
                                reload,
                                result: Err(e.clone()),
                            },
                        );
                        repaint.request_repaint();
                        UndoResult::err(format!("canvas redo (re-place block) failed: {e}"))
                    }
                }
            })
        });
        // Sync fallbacks are benign no-ops (a canvas undo MUST go through the backend — there is no
        // pure in-memory revert for a persisted placement; the async path is the real one).
        let undo_fn: UndoFn = Arc::new(UndoResult::ok);
        let redo_fn: UndoFn = Arc::new(UndoResult::ok);
        bus.push_undo_cross_pane(UndoAction::async_compensating(
            action_id,
            description,
            undo_fn,
            redo_fn,
            undo_async_fn,
            redo_async_fn,
        ));
    }

    /// Send a prebuilt canvas mutation [`crate::backend_client::RequestSpec`] for a compensating
    /// undo/redo and AWAIT the 2xx result (the board re-fetches after). Bridges the bus's async
    /// dispatch to the same verified `CanvasBoardClient` transport via its dispatch cell.
    async fn send_canvas_compensation(
        client: &CanvasBoardClient,
        spec: crate::backend_client::RequestSpec,
    ) -> Result<(), String> {
        let cell: crate::backend_client::CanvasBoardOpCell = Arc::new(std::sync::Mutex::new(None));
        client.dispatch(spec, cell.clone());
        // Poll the delivery cell briefly; the dispatch spawned its own task on the runtime. A short
        // bounded wait keeps this honest (no infinite spin) while letting the spawned request land.
        for _ in 0..600 {
            if let Ok(slot) = cell.lock() {
                if let Some(result) = slot.as_ref() {
                    return result.clone();
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Err("canvas compensating call timed out (no 2xx within bound)".to_owned())
    }

    async fn send_canvas_created_compensation(
        client: &CanvasBoardClient,
        spec: crate::backend_client::RequestSpec,
    ) -> Result<CreatedCanvasPlacement, CanvasCreateReceiptError> {
        client.create_placement_now(spec).await
    }
}

#[cfg(test)]
mod tests {
    use super::interop_adapter::{
        commit_redo_identity_then_publish, fail_closed_redo_receipt, CanvasPlacementUndoState,
    };
    use crate::backend_client::{
        validate_created_placement_receipt, CanvasCreateReceiptError, CreatedCanvasPlacement,
    };

    #[test]
    fn canvas_redo_updates_current_placement_id_for_next_undo() {
        let state = CanvasPlacementUndoState::new("placement-original".to_owned());
        assert_eq!(
            state.current_placement_id(),
            "placement-original",
            "first undo targets the originally created placement"
        );

        state.accept_redo_created_placement(CreatedCanvasPlacement {
            placement_id: "placement-redone".to_owned(),
            placed_block_id: "block-1".to_owned(),
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 120.0,
            created_by_request: true,
        });

        assert_eq!(
            state.current_placement_id(),
            "placement-redone",
            "second undo after redo targets the backend-minted replacement id"
        );
    }
    #[test]
    fn redo_replacement_identity_is_committed_before_success_is_published() {
        let state = CanvasPlacementUndoState::new("placement-original".to_owned());
        let observer = state.clone();
        commit_redo_identity_then_publish(
            &state,
            CreatedCanvasPlacement {
                placement_id: "placement-redone".to_owned(),
                placed_block_id: "block-1".to_owned(),
                x: 10.0,
                y: 20.0,
                w: 200.0,
                h: 120.0,
                created_by_request: true,
            },
            || {
                assert_eq!(
                    observer.current_placement_id(),
                    "placement-redone",
                    "success must stay hidden until the next undo target is replaced"
                );
            },
        );
    }
    #[test]
    fn every_failed_redo_receipt_stays_unowned_without_an_operation_token() {
        let failures = [
            CanvasCreateReceiptError::Rejected("409 Conflict".to_owned()),
            CanvasCreateReceiptError::Transport("connection reset".to_owned()),
            CanvasCreateReceiptError::MalformedSuccess("missing placement_id".to_owned()),
        ];
        assert!(failures
            .into_iter()
            .all(|failure| fail_closed_redo_receipt(failure).is_err()));
    }

    #[test]
    fn well_shaped_but_mismatched_create_receipt_is_rejected() {
        let request = serde_json::json!({
            "placed_block_id": "owned-block",
            "x": 10.0, "y": 20.0, "w": 200.0, "h": 120.0,
        });
        let foreign = CreatedCanvasPlacement {
            placement_id: "foreign-placement".to_owned(),
            placed_block_id: "foreign-block".to_owned(),
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 120.0,
            created_by_request: true,
        };
        let response = serde_json::json!({
            "placement_id": "foreign-placement",
            "workspace_id": "ws",
            "canvas_block_id": "canvas",
            "placed_block_id": "foreign-block",
            "x": 10.0, "y": 20.0, "w": 200.0, "h": 120.0,
        });
        let url = "http://test/workspaces/ws/loom/canvas-boards/canvas/placements";
        assert!(validate_created_placement_receipt(url, &request, &response, &foreign).is_err());

        let wrong_geometry = CreatedCanvasPlacement {
            placed_block_id: "owned-block".to_owned(),
            x: 11.0,
            ..foreign
        };
        assert!(
            validate_created_placement_receipt(url, &request, &response, &wrong_geometry).is_err()
        );

        let right_values_wrong_canvas = CreatedCanvasPlacement {
            placement_id: "other-canvas-placement".to_owned(),
            placed_block_id: "owned-block".to_owned(),
            x: 10.0,
            y: 20.0,
            w: 200.0,
            h: 120.0,
            created_by_request: true,
        };
        let wrong_scope_response = serde_json::json!({
            "placement_id": "other-canvas-placement",
            "workspace_id": "ws",
            "canvas_block_id": "other-canvas",
            "placed_block_id": "owned-block",
            "x": 10.0, "y": 20.0, "w": 200.0, "h": 120.0,
        });
        assert!(validate_created_placement_receipt(
            url,
            &request,
            &wrong_scope_response,
            &right_values_wrong_canvas,
        )
        .is_err());
    }

    #[test]
    fn wrapped_card_receipt_proves_scope_block_and_geometry() {
        let request = serde_json::json!({
            "title": "Card",
            "x": 5.0, "y": 6.0, "w": 220.0, "h": 140.0,
        });
        let response = serde_json::json!({
            "block": { "block_id": "card-block" },
            "placement": {
                "placement_id": "card-placement",
                "workspace_id": "ws",
                "canvas_block_id": "canvas",
                "placed_block_id": "card-block",
                "x": 5.0, "y": 6.0, "w": 220.0, "h": 140.0
            }
        });
        let created = CreatedCanvasPlacement {
            placement_id: "card-placement".to_owned(),
            placed_block_id: "card-block".to_owned(),
            x: 5.0,
            y: 6.0,
            w: 220.0,
            h: 140.0,
            created_by_request: true,
        };
        assert!(validate_created_placement_receipt(
            "http://test/workspaces/ws/loom/canvas-boards/canvas/cards",
            &request,
            &response,
            &created,
        )
        .is_ok());
    }
}
