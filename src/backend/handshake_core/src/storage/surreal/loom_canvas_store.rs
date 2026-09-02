//! Embedded SurrealDB persistence for Loom CanvasBoard.
//!
//! Canvas placements are references to Loom blocks, never content copies. The
//! board and placement-removal receipts are committed in the same embedded
//! transaction as their authoritative projection mutation.

use serde_json::{json, Value};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{event_ledger, loom_store, SurrealStorage, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{
    LoomBlock, LoomBlockContentType, LoomCanvasBoard, LoomCanvasBoardView, LoomCanvasPlacement,
    LoomCanvasPlacementUpdate, LoomCanvasVisualEdge, NewLoomCanvasPlacement, StorageError,
    StorageResult, WriteActorKind, WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID,
};

const WORKSPACES: &str = "workspaces";
const BLOCKS: &str = "loom_blocks";
const BOARDS: &str = "loom_canvas_boards";
const PLACEMENTS: &str = "loom_canvas_placements";
const VISUAL_EDGES: &str = "loom_canvas_visual_edges";
const EVENT_LEDGER: &str = "kernel_event_ledger";

/// The embedded engine is single-process. This lock replaces the removed
/// transaction-advisory-lock domain and serializes Canvas mutations.
static CANVAS_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(SurrealValue)]
struct BoardLookupBindings {
    board: RecordId,
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct BoardWriteBindings {
    board: RecordId,
    block: RecordId,
    workspace: RecordId,
    board_state: Value,
    expected_event: Option<RecordId>,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct MutationEventRow {
    event_id: String,
    event_sequence: i64,
}

#[derive(SurrealValue)]
struct BoardRow {
    block_id: RecordId,
    workspace_id: RecordId,
    board_state: Value,
    created_at: Datetime,
    updated_at: Datetime,
    event_ledger_event_id: RecordId,
}

#[derive(SurrealValue)]
struct PlacementLookupBindings {
    workspace: RecordId,
    canvas: RecordId,
}

#[derive(SurrealValue)]
struct PlacementWriteBindings {
    placement: RecordId,
    placement_id: String,
    canvas: RecordId,
    workspace: RecordId,
    placed_block: RecordId,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    z_index: i64,
    group_id: Option<String>,
}

#[derive(SurrealValue)]
struct PlacementUpdateBindings {
    placement: RecordId,
    workspace: RecordId,
    x: Option<f64>,
    y: Option<f64>,
    w: Option<f64>,
    h: Option<f64>,
    z_index: Option<i64>,
    group_id_set: bool,
    group_id: Option<String>,
}

#[derive(SurrealValue)]
struct RecordWorkspaceBindings {
    record: RecordId,
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct PlacementRemovalBindings {
    placement: RecordId,
    workspace: RecordId,
    canvas: RecordId,
    placed_block: RecordId,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct PlacementRow {
    placement_id: String,
    canvas_block_id: RecordId,
    workspace_id: RecordId,
    placed_block_id: RecordId,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    z_index: i64,
    group_id: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct VisualEdgeWriteBindings {
    edge: RecordId,
    visual_edge_id: String,
    workspace: RecordId,
    canvas: RecordId,
    from_placement: RecordId,
    to_placement: RecordId,
    label: Option<String>,
}

#[derive(SurrealValue)]
struct VisualEdgeRow {
    visual_edge_id: String,
    canvas_block_id: RecordId,
    workspace_id: RecordId,
    from_placement_id: RecordId,
    to_placement_id: RecordId,
    label: Option<String>,
    created_at: Datetime,
}

fn map_err(error: SurrealStorageError) -> StorageError {
    let rendered = error.to_string();
    if rendered.contains("HSK-CANVAS-BOARD-NOT-FOUND") {
        StorageError::NotFound("loom_canvas_board")
    } else if rendered.contains("HSK-CANVAS-STALE-VIEWPORT") {
        StorageError::Conflict("loom_canvas_board_stale_event_revision")
    } else if rendered.contains("HSK-CANVAS-PLACEMENT-NOT-FOUND") {
        StorageError::NotFound("loom_canvas_placement")
    } else if rendered.contains("HSK-CANVAS-VISUAL-EDGE-NOT-FOUND") {
        StorageError::NotFound("loom_canvas_visual_edge")
    } else if rendered.contains("HSK-CANVAS-WORKSPACE") {
        StorageError::Validation("canvas placement requires same-workspace board and block")
    } else if rendered.contains("HSK-CANVAS-BLOCK-TYPE") {
        StorageError::Validation("canvas board block must be content_type=canvas")
    } else if rendered.contains("HSK-CANVAS-BOARD-IDENTITY") {
        StorageError::Conflict("loom canvas board workspace identity mismatch")
    } else if rendered.contains("HSK-CANVAS-VISUAL-ENDPOINT") {
        StorageError::Validation("canvas visual edge endpoints must be placements on this canvas")
    } else {
        StorageError::Database(rendered)
    }
}

fn record_key(record: RecordId, expected_table: &'static str) -> StorageResult<String> {
    if record.table.as_str() != expected_table {
        return Err(StorageError::Serialization(format!(
            "expected {expected_table} record link, got {}",
            record.table.as_str()
        )));
    }
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Serialization(format!(
            "{expected_table} record link is not a string key"
        ))),
    }
}

fn board_to_domain(row: BoardRow) -> StorageResult<LoomCanvasBoard> {
    Ok(LoomCanvasBoard {
        block_id: record_key(row.block_id, BLOCKS)?,
        workspace_id: record_key(row.workspace_id, WORKSPACES)?,
        board_state: row.board_state,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
        event_ledger_event_id: record_key(row.event_ledger_event_id, EVENT_LEDGER)?,
    })
}

fn placement_to_domain(row: PlacementRow) -> StorageResult<LoomCanvasPlacement> {
    Ok(LoomCanvasPlacement {
        placement_id: row.placement_id,
        canvas_block_id: record_key(row.canvas_block_id, BOARDS)?,
        workspace_id: record_key(row.workspace_id, WORKSPACES)?,
        placed_block_id: record_key(row.placed_block_id, BLOCKS)?,
        x: row.x,
        y: row.y,
        w: row.w,
        h: row.h,
        z_index: i32::try_from(row.z_index)
            .map_err(|_| StorageError::Serialization("canvas z_index exceeds i32".to_owned()))?,
        group_id: row.group_id,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn visual_edge_to_domain(row: VisualEdgeRow) -> StorageResult<LoomCanvasVisualEdge> {
    Ok(LoomCanvasVisualEdge {
        visual_edge_id: row.visual_edge_id,
        canvas_block_id: record_key(row.canvas_block_id, BOARDS)?,
        workspace_id: record_key(row.workspace_id, WORKSPACES)?,
        from_placement_id: record_key(row.from_placement_id, PLACEMENTS)?,
        to_placement_id: record_key(row.to_placement_id, PLACEMENTS)?,
        label: row.label,
        created_at: row.created_at.into_inner(),
    })
}

fn validate_board_state(board_state: &Value) -> StorageResult<()> {
    let Some(object) = board_state.as_object() else {
        return Err(StorageError::Validation(
            "loom canvas board_state must be a JSON object",
        ));
    };
    if object.get("schema_id").and_then(Value::as_str) != Some(LOOM_CANVAS_BOARD_SCHEMA_ID) {
        return Err(StorageError::Validation(
            "loom canvas board_state schema_id must be hsk.loom_canvas_board@1",
        ));
    }
    let (Some(pan_x), Some(pan_y), Some(zoom)) = (
        object.get("pan_x").and_then(Value::as_f64),
        object.get("pan_y").and_then(Value::as_f64),
        object.get("zoom").and_then(Value::as_f64),
    ) else {
        return Err(StorageError::Validation(
            "loom canvas board_state requires numeric pan_x, pan_y, zoom",
        ));
    };
    if !pan_x.is_finite() || !pan_y.is_finite() || !zoom.is_finite() || zoom <= 0.0 {
        return Err(StorageError::Validation(
            "loom canvas board_state pan/zoom must be finite and zoom > 0",
        ));
    }
    Ok(())
}

fn validate_geometry(w: f64, h: f64) -> StorageResult<()> {
    if !w.is_finite() || !h.is_finite() || w <= 0.0 || h <= 0.0 {
        return Err(StorageError::Validation(
            "canvas placement w/h must be positive",
        ));
    }
    Ok(())
}

fn bridge_actor(ctx: &WriteContext) -> KernelActor {
    let actor_id = ctx
        .actor_id
        .clone()
        .unwrap_or_else(|| "loom_block_knowledge_bridge".to_owned());
    match ctx.actor_kind {
        WriteActorKind::Human => KernelActor::Operator(actor_id),
        WriteActorKind::Ai => KernelActor::ModelAdapter(actor_id),
        WriteActorKind::System => KernelActor::System(actor_id),
    }
}

fn prepare_canvas_event(
    block_id: &str,
    workspace_id: &str,
    operation: &'static str,
    board_state: Value,
) -> StorageResult<event_ledger::LedgerWrite> {
    let run_id = format!("LOOM-CANVAS-BOARD-{block_id}");
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomCanvasBoardRecorded,
        KernelActor::System("loom-canvas-board".to_owned()),
    )
    .aggregate("loom_canvas_board", block_id.to_owned())
    .source_component("loom_canvas_board")
    .payload(json!({
        "type": "knowledge_loom_canvas_board_recorded",
        "op": operation,
        "workspace_id": workspace_id,
        "block_id": block_id,
        "board_state": board_state,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom canvas EventLedger receipt build failed"))?;
    event_ledger::prepare_event(event).map(|(_, write)| write)
}

fn prepare_placement_removal_event(
    ctx: &WriteContext,
    placement: &LoomCanvasPlacement,
) -> StorageResult<event_ledger::LedgerWrite> {
    let run_id = format!("LOOM-CANVAS-PLACEMENT-{}", placement.placement_id);
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomCanvasBoardRecorded,
        bridge_actor(ctx),
    )
    .aggregate("loom_canvas_placement", placement.placement_id.clone())
    .source_component("loom_canvas_board")
    .payload(json!({
        "type": "knowledge_loom_canvas_placement_removed",
        "op": "remove_placement",
        "workspace_id": placement.workspace_id,
        "canvas_block_id": placement.canvas_block_id,
        "placement_id": placement.placement_id,
        "placed_block_id": placement.placed_block_id,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom canvas placement removal event build failed"))?;
    event_ledger::prepare_event(event).map(|(_, write)| write)
}

async fn validate_write(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    resource_id: &str,
) -> StorageResult<crate::storage::MutationMetadata> {
    storage
        .inner
        .guard
        .validate_write(ctx, resource_id)
        .await
        .map_err(StorageError::from)
}

async fn read_loom_block(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<LoomBlock> {
    let workspace_id = workspace_id.to_owned();
    let block_id = block_id.to_owned();
    storage
        .with_storage_operation(move |database| {
            Box::pin(async move {
                loom_store::get_loom_block(&database, &workspace_id, &block_id).await
            })
        })
        .await
        .map_err(StorageError::from)?
}

pub(crate) async fn create_canvas_board(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    block_id: &str,
    board_state: Value,
) -> StorageResult<LoomCanvasBoard> {
    validate_board_state(&board_state)?;
    validate_write(storage, ctx, block_id).await?;
    let block = read_loom_block(storage, workspace_id, block_id).await?;
    if !matches!(block.content_type, LoomBlockContentType::Canvas) {
        return Err(StorageError::Validation(
            "canvas board block must be content_type=canvas",
        ));
    }
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let event = prepare_canvas_event(block_id, workspace_id, "create", board_state.clone())?;
    let bindings = BoardWriteBindings {
        board: RecordId::new(BOARDS, block_id.to_owned()),
        block: RecordId::new(BLOCKS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        board_state,
        expected_event: None,
        event,
    };
    // Result indexes: BEGIN=0, block/board-identity guards=1..2, event=3,
    // UPSERT=4, COMMIT=5, projection SELECT=6.
    let rows: Vec<BoardRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $block WHERE workspace_id = $workspace \
                           AND content_type = 'canvas')[0] = NONE { \
                           THROW 'HSK-CANVAS-BLOCK-TYPE'; \
                         }; \
                         IF (SELECT VALUE id FROM $board)[0] != NONE \
                           AND (SELECT VALUE id FROM $board WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-BOARD-IDENTITY'; \
                         }; \
                         CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, \
                           actor_id: $event.actor_id, causation_id: $event.causation_id, \
                           correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                           source_component: $event.source_component, payload: $event.payload, \
                           created_at: $event.created_at \
                         }; \
                         UPSERT $board SET block_id = $block, workspace_id = $workspace, \
                           board_state = $board_state, updated_at = time::now(), \
                           event_ledger_event_id = $event.record; \
                         COMMIT TRANSACTION; \
                         SELECT block_id, workspace_id, board_state, created_at, updated_at, \
                           event_ledger_event_id FROM $board;",
                        bindings,
                        6,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .map(board_to_domain)
        .transpose()?
        .ok_or_else(|| StorageError::Database("canvas board write returned no row".to_owned()))
}

pub(crate) async fn get_canvas_board(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<LoomCanvasBoardView> {
    let bindings = BoardLookupBindings {
        board: RecordId::new(BOARDS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    let board: Option<BoardRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT block_id, workspace_id, board_state, created_at, updated_at, \
                           event_ledger_event_id FROM $board WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let board = board
        .map(board_to_domain)
        .transpose()?
        .ok_or(StorageError::NotFound("loom_canvas_board"))?;

    let bindings = PlacementLookupBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        canvas: RecordId::new(BOARDS, block_id.to_owned()),
    };
    let placements: Vec<PlacementRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, created_at, updated_at \
                         FROM loom_canvas_placements WHERE workspace_id = $workspace \
                           AND canvas_block_id = $canvas \
                         ORDER BY z_index ASC, created_at ASC, placement_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;

    let bindings = PlacementLookupBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        canvas: RecordId::new(BOARDS, block_id.to_owned()),
    };
    let visual_edges: Vec<VisualEdgeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT visual_edge_id, canvas_block_id, workspace_id, from_placement_id, \
                           to_placement_id, label, created_at FROM loom_canvas_visual_edges \
                         WHERE workspace_id = $workspace AND canvas_block_id = $canvas \
                         ORDER BY created_at ASC, visual_edge_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;

    Ok(LoomCanvasBoardView {
        board,
        placements: placements
            .into_iter()
            .map(placement_to_domain)
            .collect::<StorageResult<_>>()?,
        visual_edges: visual_edges
            .into_iter()
            .map(visual_edge_to_domain)
            .collect::<StorageResult<_>>()?,
    })
}

pub(crate) async fn update_canvas_board_state(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    block_id: &str,
    board_state: Value,
) -> StorageResult<LoomCanvasBoard> {
    validate_board_state(&board_state)?;
    validate_write(storage, ctx, block_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let expected_event_ledger_event_id = get_canvas_board(storage, workspace_id, block_id)
        .await?
        .board
        .event_ledger_event_id;
    let event = prepare_canvas_event(block_id, workspace_id, "viewport", board_state.clone())?;
    let bindings = BoardWriteBindings {
        board: RecordId::new(BOARDS, block_id.to_owned()),
        block: RecordId::new(BLOCKS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        board_state,
        expected_event: Some(RecordId::new(EVENT_LEDGER, expected_event_ledger_event_id)),
        event,
    };
    // Result indexes: BEGIN=0, board guard=1, revision guard=2, event=3,
    // UPDATE=4, COMMIT=5, projection SELECT=6.
    let rows: Vec<BoardRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $board WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-BOARD-NOT-FOUND'; \
                         }; \
                         IF (SELECT VALUE event_ledger_event_id FROM $board \
                           WHERE workspace_id = $workspace)[0] != $expected_event { \
                           THROW 'HSK-CANVAS-STALE-VIEWPORT'; \
                         }; \
                         CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, \
                           actor_id: $event.actor_id, causation_id: $event.causation_id, \
                           correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                           source_component: $event.source_component, payload: $event.payload, \
                           created_at: $event.created_at \
                         }; \
                         UPDATE $board SET board_state = $board_state, updated_at = time::now(), \
                           event_ledger_event_id = $event.record; \
                         COMMIT TRANSACTION; \
                         SELECT block_id, workspace_id, board_state, created_at, updated_at, \
                           event_ledger_event_id FROM $board;",
                        bindings,
                        6,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .map(board_to_domain)
        .transpose()?
        .ok_or_else(|| StorageError::Database("canvas board update returned no row".to_owned()))
}

pub(crate) async fn place_block_on_canvas(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    placement: NewLoomCanvasPlacement,
) -> StorageResult<LoomCanvasPlacement> {
    validate_geometry(placement.w, placement.h)?;
    let placement_id = format!("LCP-{}", Uuid::now_v7().simple());
    validate_write(storage, ctx, &placement_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let bindings = PlacementWriteBindings {
        placement: RecordId::new(PLACEMENTS, placement_id.clone()),
        placement_id,
        canvas: RecordId::new(BOARDS, placement.canvas_block_id),
        workspace: RecordId::new(WORKSPACES, placement.workspace_id),
        placed_block: RecordId::new(BLOCKS, placement.placed_block_id),
        x: placement.x,
        y: placement.y,
        w: placement.w,
        h: placement.h,
        z_index: i64::from(placement.z_index),
        group_id: placement.group_id,
    };
    // Result indexes: BEGIN=0, board guard=1, block guard=2, CREATE=3,
    // COMMIT=4, projection SELECT=5.
    let rows: Vec<PlacementRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $canvas WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-WORKSPACE'; \
                         }; \
                         IF (SELECT VALUE id FROM $placed_block WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-WORKSPACE'; \
                         }; \
                         CREATE $placement CONTENT { placement_id: $placement_id, \
                           canvas_block_id: $canvas, workspace_id: $workspace, placed_block_id: $placed_block, \
                           x: $x, y: $y, w: $w, h: $h, z_index: $z_index, group_id: $group_id };
                         COMMIT TRANSACTION; \
                         SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, created_at, updated_at \
                         FROM $placement;",
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .map(placement_to_domain)
        .transpose()?
        .ok_or_else(|| StorageError::Database("canvas placement create returned no row".to_owned()))
}

pub(crate) async fn update_canvas_placement(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    placement_id: &str,
    update: LoomCanvasPlacementUpdate,
) -> StorageResult<LoomCanvasPlacement> {
    if let Some(w) = update.w {
        validate_geometry(w, update.h.unwrap_or(1.0))?;
    } else if let Some(h) = update.h {
        validate_geometry(1.0, h)?;
    }
    validate_write(storage, ctx, placement_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let group_id_set = update.group_id.is_some();
    let bindings = PlacementUpdateBindings {
        placement: RecordId::new(PLACEMENTS, placement_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        x: update.x,
        y: update.y,
        w: update.w,
        h: update.h,
        z_index: update.z_index.map(i64::from),
        group_id_set,
        group_id: update.group_id.flatten(),
    };
    let rows: Vec<PlacementRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "IF (SELECT VALUE id FROM $placement WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-PLACEMENT-NOT-FOUND'; \
                         }; \
                         UPDATE $placement SET x = $x ?? x, y = $y ?? y, w = $w ?? w, h = $h ?? h, \
                           z_index = $z_index ?? z_index, \
                           group_id = IF $group_id_set { $group_id } ELSE { group_id }, \
                           updated_at = time::now(); \
                         SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, created_at, updated_at \
                         FROM $placement;",
                        bindings,
                        2,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .map(placement_to_domain)
        .transpose()?
        .ok_or_else(|| StorageError::Database("canvas placement update returned no row".to_owned()))
}

async fn read_placement(
    storage: &SurrealStorage,
    workspace_id: &str,
    placement_id: &str,
) -> StorageResult<LoomCanvasPlacement> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new(PLACEMENTS, placement_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    let row: Option<PlacementRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, created_at, updated_at \
                         FROM $record WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    row.map(placement_to_domain)
        .transpose()?
        .ok_or(StorageError::NotFound("loom_canvas_placement"))
}

pub(crate) async fn remove_canvas_placement(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    placement_id: &str,
) -> StorageResult<()> {
    validate_write(storage, ctx, placement_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let placement = read_placement(storage, workspace_id, placement_id).await?;
    let event = prepare_placement_removal_event(ctx, &placement)?;
    let bindings = PlacementRemovalBindings {
        placement: RecordId::new(PLACEMENTS, placement.placement_id.clone()),
        workspace: RecordId::new(WORKSPACES, placement.workspace_id.clone()),
        canvas: RecordId::new(BOARDS, placement.canvas_block_id.clone()),
        placed_block: RecordId::new(BLOCKS, placement.placed_block_id.clone()),
        event,
    };
    // Result indexes: BEGIN=0, identity guard=1, event=2, deletion=3,
    // COMMIT=4, exact receipt SELECT=5.
    let rows: Vec<MutationEventRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF array::len((SELECT id FROM $placement WHERE workspace_id = $workspace \
                           AND canvas_block_id = $canvas AND placed_block_id = $placed_block)) != 1 { \
                           THROW 'HSK-CANVAS-PLACEMENT-NOT-FOUND'; \
                         }; \
                         CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, \
                           actor_id: $event.actor_id, causation_id: $event.causation_id, \
                           correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                           source_component: $event.source_component, payload: $event.payload, \
                           created_at: $event.created_at \
                         }; \
                         DELETE $placement; \
                         COMMIT TRANSACTION; \
                         SELECT event_id, event_sequence FROM $event.record;",
                        bindings,
                        5,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let event = rows.into_iter().next().ok_or_else(|| {
        StorageError::Database(
            "committed Canvas placement removal EventLedger row is missing".to_owned(),
        )
    })?;
    if event.event_id.trim().is_empty() || event.event_sequence < 0 {
        return Err(StorageError::Database(
            "committed Canvas placement removal EventLedger row is malformed".to_owned(),
        ));
    }
    Ok(())
}

pub(crate) async fn add_canvas_visual_edge(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    canvas_block_id: &str,
    from_placement_id: &str,
    to_placement_id: &str,
    label: Option<String>,
) -> StorageResult<LoomCanvasVisualEdge> {
    if from_placement_id == to_placement_id {
        return Err(StorageError::Validation(
            "canvas visual edge endpoints must differ",
        ));
    }
    let visual_edge_id = format!("LCV-{}", Uuid::now_v7().simple());
    validate_write(storage, ctx, &visual_edge_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let bindings = VisualEdgeWriteBindings {
        edge: RecordId::new(VISUAL_EDGES, visual_edge_id.clone()),
        visual_edge_id,
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        canvas: RecordId::new(BOARDS, canvas_block_id.to_owned()),
        from_placement: RecordId::new(PLACEMENTS, from_placement_id.to_owned()),
        to_placement: RecordId::new(PLACEMENTS, to_placement_id.to_owned()),
        label,
    };
    // Result indexes: BEGIN=0, endpoint guard=1, CREATE=2, COMMIT=3,
    // projection SELECT=4.
    let rows: Vec<VisualEdgeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF array::len((SELECT id FROM loom_canvas_placements \
                           WHERE workspace_id = $workspace AND canvas_block_id = $canvas \
                             AND id IN [$from_placement, $to_placement])) != 2 { \
                           THROW 'HSK-CANVAS-VISUAL-ENDPOINT'; \
                         }; \
                         CREATE $edge CONTENT { visual_edge_id: $visual_edge_id, \
                           canvas_block_id: $canvas, workspace_id: $workspace, \
                           from_placement_id: $from_placement, to_placement_id: $to_placement, \
                           label: $label }; \
                         COMMIT TRANSACTION; \
                         SELECT visual_edge_id, canvas_block_id, workspace_id, from_placement_id, \
                           to_placement_id, label, created_at FROM $edge;",
                        bindings,
                        4,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    rows.into_iter()
        .next()
        .map(visual_edge_to_domain)
        .transpose()?
        .ok_or_else(|| {
            StorageError::Database("canvas visual edge create returned no row".to_owned())
        })
}

pub(crate) async fn remove_canvas_visual_edge(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    visual_edge_id: &str,
) -> StorageResult<()> {
    validate_write(storage, ctx, visual_edge_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let count = storage
        .with_data_operation({
            let bindings = RecordWorkspaceBindings {
                record: RecordId::new(VISUAL_EDGES, visual_edge_id.to_owned()),
                workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .execute_returning(
                            "DELETE $record WHERE workspace_id = $workspace RETURN BEFORE;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    if count == 1 {
        Ok(())
    } else {
        Err(StorageError::NotFound("loom_canvas_visual_edge"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::surreal::{SurrealStorage, SurrealStorageConfig};
    use crate::storage::{LoomBlockDerived, MutationMetadata, NewLoomBlock, WriteActorKind};
    use chrono::Utc;

    #[derive(SurrealValue)]
    struct WorkspaceSeed {
        name: String,
    }

    #[derive(SurrealValue)]
    struct RemovalEventProofRow {
        event_id: String,
        event_sequence: i64,
        payload: Value,
    }

    #[derive(SurrealValue)]
    struct RemovalEventBinding {
        aggregate_id: String,
    }

    fn context() -> WriteContext {
        WriteContext::system(Some("loom-canvas-receipt-test".to_owned()))
    }

    fn metadata(resource_id: &str) -> MutationMetadata {
        MutationMetadata {
            actor_kind: WriteActorKind::System,
            actor_id: Some("loom-canvas-receipt-test".to_owned()),
            job_id: None,
            workflow_id: None,
            edit_event_id: Uuid::now_v7(),
            resource_id: resource_id.to_owned(),
            timestamp: Utc::now(),
        }
    }

    fn board_state(pan_x: f64) -> Value {
        json!({
            "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
            "pan_x": pan_x,
            "pan_y": 0.0,
            "zoom": 1.0,
        })
    }

    async fn open_store() -> (tempfile::TempDir, SurrealStorage) {
        let temp = tempfile::tempdir().expect("create temporary data root");
        let config = SurrealStorageConfig::for_data_dir(temp.path())
            .expect("configure real embedded Surreal store");
        let store = SurrealStorage::open(config)
            .await
            .expect("open real embedded Surreal store");
        super::super::schema::bootstrap_loom_receipt_test_schema(&store)
            .await
            .expect("bootstrap production Loom receipt schema");
        (temp, store)
    }

    async fn seed_workspace(store: &SurrealStorage, workspace_id: &str) {
        let workspace_id = workspace_id.to_owned();
        store
            .with_data_operation(move |db| {
                Box::pin(async move {
                    let _: Option<surrealdb::types::Value> = db
                        .upsert_one(
                            WORKSPACES,
                            &workspace_id,
                            WorkspaceSeed {
                                name: "Canvas receipt workspace".to_owned(),
                            },
                        )
                        .await?;
                    Ok(())
                })
            })
            .await
            .expect("seed workspace");
    }

    async fn create_block(
        store: &SurrealStorage,
        workspace_id: &str,
        block_id: &str,
        content_type: LoomBlockContentType,
    ) {
        let block = NewLoomBlock {
            block_id: Some(block_id.to_owned()),
            workspace_id: workspace_id.to_owned(),
            content_type,
            document_id: None,
            asset_id: None,
            title: Some(block_id.to_owned()),
            original_filename: None,
            content_hash: None,
            pinned: false,
            journal_date: None,
            imported_at: None,
            derived: LoomBlockDerived::default(),
        };
        let write_metadata = metadata(block_id);
        store
            .with_storage_operation(move |db| {
                Box::pin(
                    async move { loom_store::create_loom_block(&db, block, write_metadata).await },
                )
            })
            .await
            .expect("block lifecycle")
            .expect("create Loom block");
    }

    async fn create_board_fixture(
        store: &SurrealStorage,
        workspace_id: &str,
        canvas_id: &str,
    ) -> LoomCanvasBoard {
        seed_workspace(store, workspace_id).await;
        create_block(store, workspace_id, canvas_id, LoomBlockContentType::Canvas).await;
        create_canvas_board(store, &context(), workspace_id, canvas_id, board_state(0.0))
            .await
            .expect("create Canvas board")
    }

    #[tokio::test]
    async fn viewport_compare_and_swap_rejects_stale_event_revision() {
        let (_temp, store) = open_store().await;
        let workspace_id = "canvas-cas-workspace";
        let canvas_id = "canvas-cas";
        let created = create_board_fixture(&store, workspace_id, canvas_id).await;

        let updated = update_canvas_board_state(
            &store,
            &context(),
            workspace_id,
            canvas_id,
            board_state(10.0),
            &created.event_ledger_event_id,
        )
        .await
        .expect("first viewport update");
        assert_ne!(updated.event_ledger_event_id, created.event_ledger_event_id);
        assert!(updated.updated_at >= created.updated_at);

        let stale = update_canvas_board_state(
            &store,
            &context(),
            workspace_id,
            canvas_id,
            board_state(99.0),
            &created.event_ledger_event_id,
        )
        .await;
        assert!(matches!(
            stale,
            Err(StorageError::Conflict(
                "loom_canvas_board_stale_event_revision"
            ))
        ));

        let authoritative = get_canvas_board(&store, workspace_id, canvas_id)
            .await
            .expect("read authoritative Canvas board");
        assert_eq!(
            authoritative.board.event_ledger_event_id,
            updated.event_ledger_event_id
        );
        assert_eq!(authoritative.board.updated_at, updated.updated_at);
        assert_eq!(authoritative.board.board_state["pan_x"], 10.0);
        store.shutdown().await.expect("close embedded store");
    }

    #[tokio::test]
    async fn placement_removal_returns_exact_event_and_preserves_source_block() {
        let (_temp, store) = open_store().await;
        let workspace_id = "canvas-removal-workspace";
        let canvas_id = "canvas-removal";
        let source_id = "canvas-removal-source";
        create_board_fixture(&store, workspace_id, canvas_id).await;
        create_block(&store, workspace_id, source_id, LoomBlockContentType::Note).await;
        let placement = place_block_on_canvas(
            &store,
            &context(),
            NewLoomCanvasPlacement {
                canvas_block_id: canvas_id.to_owned(),
                workspace_id: workspace_id.to_owned(),
                placed_block_id: source_id.to_owned(),
                x: 1.0,
                y: 2.0,
                w: 100.0,
                h: 80.0,
                z_index: 0,
                group_id: None,
            },
        )
        .await
        .expect("place source block");

        remove_canvas_placement(&store, &context(), workspace_id, &placement.placement_id)
            .await
            .expect("remove placement");

        let event = store
            .with_data_operation({
                let aggregate_id = placement.placement_id.clone();
                move |db| {
                    Box::pin(async move {
                        db.query_first::<RemovalEventProofRow, _>(
                            "SELECT event_id, event_sequence, payload                              FROM kernel_event_ledger                              WHERE aggregate_type = 'loom_canvas_placement'                                AND aggregate_id = $aggregate_id                                AND source_component = 'loom_canvas_board'                                AND payload.op = 'remove_placement'                              ORDER BY event_sequence DESC LIMIT 1;",
                            RemovalEventBinding { aggregate_id },
                        )
                        .await
                    })
                }
            })
            .await
            .expect("read removal event")
            .expect("removal event exists");
        assert!(!event.event_id.is_empty());
        assert!(event.event_sequence >= 0);
        assert_eq!(event.payload["workspace_id"], workspace_id);
        assert_eq!(event.payload["canvas_block_id"], canvas_id);
        assert_eq!(event.payload["placement_id"], placement.placement_id);
        assert_eq!(event.payload["placed_block_id"], source_id);

        let board = get_canvas_board(&store, workspace_id, canvas_id)
            .await
            .expect("read Canvas after placement removal");
        assert!(board.placements.is_empty());
        assert_eq!(
            read_loom_block(&store, workspace_id, source_id)
                .await
                .expect("source block survives")
                .block_id,
            source_id
        );
        store.shutdown().await.expect("close embedded store");
    }
}
