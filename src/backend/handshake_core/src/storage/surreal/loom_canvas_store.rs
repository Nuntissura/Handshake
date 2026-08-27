//! Embedded SurrealDB persistence for Loom CanvasBoard.
//!
//! Canvas placements are references to Loom blocks, never content copies. The
//! Stage-card path is deliberately implemented here as one transaction because
//! its RichDocument, Loom/search projection, knowledge bridge, placement, and
//! EventLedger receipt form one compensation-owned authority tuple.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{event_ledger, loom_store, SurrealStorage, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{knowledge_canonical_json_sha256, rich_document_loom_projection};
use crate::storage::{
    CompensateLoomCanvasStageCard, LoomBlock, LoomBlockContentType, LoomCanvasBoard,
    LoomCanvasBoardView, LoomCanvasPlacement, LoomCanvasPlacementUpdate, LoomCanvasStageCard,
    LoomCanvasStageCompensation, LoomCanvasStageProvenance, LoomCanvasVisualEdge,
    NewLoomCanvasPlacement, NewLoomCanvasStageCard, StorageError, StorageResult, WriteActorKind,
    WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID, LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
};

const WORKSPACES: &str = "workspaces";
const BLOCKS: &str = "loom_blocks";
const BOARDS: &str = "loom_canvas_boards";
const PLACEMENTS: &str = "loom_canvas_placements";
const VISUAL_EDGES: &str = "loom_canvas_visual_edges";
const DOCUMENTS: &str = "knowledge_rich_documents";
const ENTITIES: &str = "knowledge_entities";
const EVENT_LEDGER: &str = "kernel_event_ledger";
const BRIDGES: &str = "loom_block_knowledge_bridge";
const EXTRACTOR_VERSION: &str = "loom_block_knowledge_bridge_v1";

/// The embedded engine is single-process. This lock replaces the removed
/// transaction-advisory-lock domain and serializes every Canvas mutation that
/// can interact with a Stage provenance key or compensation-owned placement.
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
    event: event_ledger::LedgerWrite,
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
    is_text_card: bool,
    stage_provenance_key: Option<String>,
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
    is_text_card: bool,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct StagePlacementRow {
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
    is_text_card: bool,
    stage_provenance_key: Option<String>,
    stage_provenance: Option<Value>,
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

#[derive(SurrealValue)]
struct StageAuthorityBindings {
    workspace: RecordId,
    artifact: RecordId,
}

#[derive(SurrealValue)]
struct StageAuthorityRow {
    content_sha256: String,
    manifest_ref: String,
    correlation_id: String,
}

#[derive(SurrealValue)]
struct StageKeyBindings {
    workspace: RecordId,
    canvas: RecordId,
    stage_provenance_key: String,
}

#[derive(SurrealValue)]
struct StageCreateBindings {
    workspace: RecordId,
    canvas: RecordId,
    artifact: RecordId,
    document: RecordId,
    block: RecordId,
    search: RecordId,
    document_id: String,
    document_title: String,
    schema_version: String,
    content_json: Value,
    content_sha256: String,
    derived_json: Value,
    search_text: String,
    entity: RecordId,
    entity_id: String,
    bridge: RecordId,
    placement: RecordId,
    placement_id: String,
    stage_provenance_key: String,
    stage_provenance: Value,
    provenance_sha256: String,
    provenance_manifest_ref: String,
    provenance_correlation_id: String,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    z_index: i64,
    actor_id: Option<String>,
    actor_kind: String,
    edit_event_id: String,
    written_at: Datetime,
    detection_provenance: Value,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct StageDocumentRow {
    rich_document_id: String,
    workspace_id: RecordId,
    document_id: Option<RecordId>,
    title: String,
    schema_version: String,
    doc_version: i64,
    content_json: Value,
    content_sha256: String,
    crdt_document_id: Option<String>,
    crdt_snapshot_id: Option<String>,
    promotion_receipt_event_id: Option<RecordId>,
    projection_refs: Value,
    project_ref: Option<String>,
    folder_ref: Option<String>,
    authority_label: String,
    owner_actor_kind: Option<String>,
    owner_actor_id: Option<String>,
    deleted_at: Option<Datetime>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct StageVersionRow {
    doc_version: i64,
    schema_version: String,
    content_json: Value,
    content_sha256: String,
    crdt_snapshot_id: Option<String>,
    promotion_receipt_event_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct StageBlockOwnershipRow {
    title: Option<String>,
    content_type: String,
    content_hash: Option<String>,
    document_id: Option<RecordId>,
    asset_id: Option<RecordId>,
    original_filename: Option<String>,
    pinned: bool,
    favorite: bool,
    pin_order: Option<i64>,
    journal_date: Option<String>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    imported_at: Option<Datetime>,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    derived_json: Value,
    preview_status: String,
    thumbnail_asset_id: Option<RecordId>,
    proxy_asset_id: Option<RecordId>,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct StageBridgeOwnershipRow {
    entity_id: RecordId,
    index_event_id: RecordId,
    bridge_created_at: Datetime,
    bridge_updated_at: Datetime,
    entity_kind: String,
    entity_key: String,
    display_name: String,
    detection_provenance: Value,
    primary_source_id: Option<RecordId>,
    first_detected_in_run: Option<RecordId>,
    last_detected_in_run: Option<RecordId>,
    lifecycle_state: String,
    entity_created_at: Datetime,
    entity_updated_at: Datetime,
    index_event_type: String,
    index_aggregate_type: String,
    index_aggregate_id: String,
    index_source_component: String,
    index_payload: Value,
}

#[derive(SurrealValue)]
struct SearchOwnershipRow {
    workspace_id: RecordId,
    content_type: String,
    search_text: String,
    embedding: Option<Vec<f64>>,
    embedding_model: Option<String>,
}

#[derive(SurrealValue)]
struct StageReceiptBindings {
    placement: RecordId,
    workspace: RecordId,
    canvas: RecordId,
    block: RecordId,
    document: RecordId,
    block_id: String,
    expected_title: String,
    stage_provenance_key: String,
}

#[derive(SurrealValue)]
struct PresenceRow {
    present: bool,
}

#[derive(SurrealValue)]
struct StageCompensationBindings {
    placement: RecordId,
    workspace: RecordId,
    canvas: RecordId,
    block: RecordId,
    document: RecordId,
    search: RecordId,
    bridge: RecordId,
    entity: RecordId,
    entity_id: String,
    index_event: RecordId,
    block_id: String,
    expected_title: String,
    schema_version: String,
    content_json: Value,
    content_sha256: String,
    derived_json: Value,
    search_text: String,
    stage_provenance_key: String,
    stage_provenance: Value,
    detection_provenance: Value,
    index_payload: Value,
    event: event_ledger::LedgerWrite,
}

fn map_err(error: SurrealStorageError) -> StorageError {
    let rendered = error.to_string();
    if rendered.contains("HSK-CANVAS-BOARD-NOT-FOUND") {
        StorageError::NotFound("loom_canvas_board")
    } else if rendered.contains("HSK-CANVAS-PLACEMENT-NOT-FOUND") {
        StorageError::NotFound("loom_canvas_placement")
    } else if rendered.contains("HSK-CANVAS-VISUAL-EDGE-NOT-FOUND") {
        StorageError::NotFound("loom_canvas_visual_edge")
    } else if rendered.contains("HSK-CANVAS-STAGE-AUTHORITY") {
        StorageError::Validation(
            "Canvas Stage provenance does not match the authoritative capture tuple",
        )
    } else if rendered.contains("HSK-CANVAS-STAGE-PROVENANCE-CONFLICT") {
        StorageError::Validation("Canvas Stage provenance key is bound to a different tuple")
    } else if rendered.contains("HSK-CANVAS-STAGE-COMPENSATION") {
        StorageError::Validation("Canvas Stage compensation ownership changed during commit")
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
        is_text_card: row.is_text_card,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

fn stage_placement_to_domain(row: &StagePlacementRow) -> StorageResult<LoomCanvasPlacement> {
    placement_to_domain(PlacementRow {
        placement_id: row.placement_id.clone(),
        canvas_block_id: row.canvas_block_id.clone(),
        workspace_id: row.workspace_id.clone(),
        placed_block_id: row.placed_block_id.clone(),
        x: row.x,
        y: row.y,
        w: row.w,
        h: row.h,
        z_index: row.z_index,
        group_id: row.group_id.clone(),
        is_text_card: row.is_text_card,
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
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

fn validated_stage_provenance(
    key: &str,
    provenance: &LoomCanvasStageProvenance,
) -> StorageResult<Value> {
    if provenance.schema_id != LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA
        || provenance.artifact_id.trim().is_empty()
        || provenance.artifact_id.trim() != provenance.artifact_id
        || provenance.manifest_ref.trim().is_empty()
        || provenance.manifest_ref.trim() != provenance.manifest_ref
        || provenance.causal_action_id.trim().is_empty()
        || provenance.causal_action_id.trim() != provenance.causal_action_id
        || provenance.sha256.len() != 64
        || !provenance
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(StorageError::Validation(
            "invalid Canvas Stage provenance tuple",
        ));
    }
    let canonical = serde_json::to_vec(provenance)?;
    let computed_key = format!("{:x}", Sha256::digest(canonical));
    if key != computed_key {
        return Err(StorageError::Validation(
            "Canvas Stage provenance key does not match the exact tuple",
        ));
    }
    Ok(serde_json::to_value(provenance)?)
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
                           x, y, w, h, z_index, group_id, is_text_card, created_at, updated_at \
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
    let event = prepare_canvas_event(block_id, workspace_id, "viewport", board_state.clone())?;
    let bindings = BoardWriteBindings {
        board: RecordId::new(BOARDS, block_id.to_owned()),
        block: RecordId::new(BLOCKS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        board_state,
        event,
    };
    // Result indexes: BEGIN=0, board guard=1, event=2, UPDATE=3,
    // COMMIT=4, projection SELECT=5.
    let rows: Vec<BoardRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $board WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-BOARD-NOT-FOUND'; \
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
                        5,
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
    if placement.stage_provenance_key.is_some() {
        return Err(StorageError::Validation(
            "Stage provenance placements must use create_stage_canvas_card",
        ));
    }
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
        is_text_card: placement.is_text_card,
        stage_provenance_key: placement.stage_provenance_key,
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
                           x: $x, y: $y, w: $w, h: $h, z_index: $z_index, group_id: $group_id, \
                           is_text_card: $is_text_card, stage_provenance_key: $stage_provenance_key, \
                           stage_provenance: NONE };
                         COMMIT TRANSACTION; \
                         SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, is_text_card, created_at, updated_at \
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

async fn read_stage_authority(
    storage: &SurrealStorage,
    workspace_id: &str,
    artifact_id: &str,
) -> StorageResult<Option<StageAuthorityRow>> {
    let bindings = StageAuthorityBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        artifact: RecordId::new("stage_capture_artifacts", artifact_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT content_sha256, manifest_ref, correlation_id FROM $artifact \
                         WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn read_stage_placements(
    storage: &SurrealStorage,
    workspace_id: &str,
    canvas_block_id: &str,
    stage_provenance_key: &str,
) -> StorageResult<Vec<StagePlacementRow>> {
    let bindings = StageKeyBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        canvas: RecordId::new(BOARDS, canvas_block_id.to_owned()),
        stage_provenance_key: stage_provenance_key.to_owned(),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, is_text_card, stage_provenance_key, \
                           stage_provenance, created_at, updated_at FROM loom_canvas_placements \
                         WHERE workspace_id = $workspace AND canvas_block_id = $canvas \
                           AND stage_provenance_key = $stage_provenance_key \
                         ORDER BY placement_id ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn stage_replay(
    storage: &SurrealStorage,
    card: &NewLoomCanvasStageCard,
    expected_provenance: &Value,
    placement: StagePlacementRow,
) -> StorageResult<LoomCanvasStageCard> {
    if placement.stage_provenance_key.as_deref() != Some(card.stage_provenance_key.as_str())
        || placement.stage_provenance.as_ref() != Some(expected_provenance)
        || !placement.is_text_card
    {
        return Err(StorageError::Validation(
            "Canvas Stage provenance key is bound to a different tuple",
        ));
    }
    let placed_block_id = record_key(placement.placed_block_id.clone(), BLOCKS)?;
    let document: Option<StageDocumentRow> = storage
        .with_data_operation({
            let bindings = RecordWorkspaceBindings {
                record: RecordId::new(DOCUMENTS, placed_block_id.clone()),
                workspace: RecordId::new(WORKSPACES, card.workspace_id.clone()),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT rich_document_id, workspace_id, document_id, title, schema_version, \
                               doc_version, content_json, content_sha256, crdt_document_id, \
                               crdt_snapshot_id, promotion_receipt_event_id, projection_refs, project_ref, \
                               folder_ref, authority_label, owner_actor_kind, owner_actor_id, deleted_at, \
                               created_at, updated_at FROM $record WHERE workspace_id = $workspace \
                               AND deleted_at = NONE;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    let document = document.ok_or(StorageError::Validation(
        "Canvas Stage provenance authority tuple is incomplete",
    ))?;
    if document.rich_document_id != placed_block_id || document.title != card.title {
        return Err(StorageError::Validation(
            "Canvas Stage provenance key is bound to a different tuple",
        ));
    }
    let block = read_loom_block(storage, &card.workspace_id, &placed_block_id).await?;
    Ok(LoomCanvasStageCard {
        block,
        rich_document_id: document.rich_document_id,
        placement: stage_placement_to_domain(&placement)?,
        created_by_request: false,
    })
}

pub(crate) async fn create_stage_canvas_card(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    card: NewLoomCanvasStageCard,
) -> StorageResult<LoomCanvasStageCard> {
    use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
    use crate::knowledge_document::import::{import_snippet, ImportFormat};

    if card.title.trim() != card.title || card.title.is_empty() {
        return Err(StorageError::Validation(
            "Stage canvas card title must be non-empty and trimmed",
        ));
    }
    validate_geometry(card.w, card.h)?;
    let stage_provenance =
        validated_stage_provenance(&card.stage_provenance_key, &card.stage_provenance)?;
    let canonical_markdown = serde_json::to_string(&card.stage_provenance)?;
    let imported = import_snippet(&canonical_markdown, ImportFormat::Markdown);

    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;
    let authority = read_stage_authority(
        storage,
        &card.workspace_id,
        &card.stage_provenance.artifact_id,
    )
    .await?
    .ok_or(StorageError::Validation(
        "Canvas Stage provenance has no authoritative capture artifact",
    ))?;
    if authority.content_sha256 != card.stage_provenance.sha256
        || authority.manifest_ref != card.stage_provenance.manifest_ref
        || authority.correlation_id != card.stage_provenance.causal_action_id
    {
        return Err(StorageError::Validation(
            "Canvas Stage provenance does not match the authoritative capture tuple",
        ));
    }

    let existing = read_stage_placements(
        storage,
        &card.workspace_id,
        &card.canvas_block_id,
        &card.stage_provenance_key,
    )
    .await?;
    match existing.len() {
        0 => {}
        1 => {
            return stage_replay(
                storage,
                &card,
                &stage_provenance,
                existing.into_iter().next().expect("one row"),
            )
            .await;
        }
        _ => {
            return Err(StorageError::Conflict(
                "duplicate Canvas Stage provenance key",
            ));
        }
    }

    let document_id = format!("KRD-{}", Uuid::now_v7().simple());
    let entity_id = format!("KEN-{}", Uuid::now_v7().simple());
    let placement_id = format!("LCP-{}", Uuid::now_v7().simple());
    let document_metadata = validate_write(storage, ctx, &document_id).await?;
    validate_write(storage, ctx, &placement_id).await?;
    let content_sha256 = knowledge_canonical_json_sha256(&imported.document_json);
    let (derived_json, search_text) =
        rich_document_loom_projection(&card.title, &imported.document_json)?;
    let derived_json: Value = serde_json::from_str(&derived_json)?;
    let actor = bridge_actor(ctx);
    let run_id = format!("LOOM-BRIDGE-{}", card.workspace_id);
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomBlockIndexed,
        actor,
    )
    .aggregate("knowledge_loom_block", entity_id.clone())
    .idempotency_key(format!(
        "KEI-loom-bridge-{}-{}",
        entity_id,
        document_metadata
            .timestamp
            .timestamp_nanos_opt()
            .unwrap_or_default()
    ))
    .source_component("loom_block_knowledge_bridge")
    .payload(json!({
        "type": "knowledge_loom_block_indexed",
        "workspace_id": card.workspace_id,
        "block_id": document_id,
        "entity_id": entity_id,
        "content_type": "note",
        "extractor_version": EXTRACTOR_VERSION,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom bridge EventLedger receipt build failed"))?;
    let (_, event) = event_ledger::prepare_event(event)?;
    let bindings = StageCreateBindings {
        workspace: RecordId::new(WORKSPACES, card.workspace_id.clone()),
        canvas: RecordId::new(BOARDS, card.canvas_block_id.clone()),
        artifact: RecordId::new(
            "stage_capture_artifacts",
            card.stage_provenance.artifact_id.clone(),
        ),
        document: RecordId::new(DOCUMENTS, document_id.clone()),
        block: RecordId::new(BLOCKS, document_id.clone()),
        search: RecordId::new("loom_block_search_index", document_id.clone()),
        document_id: document_id.clone(),
        document_title: card.title.clone(),
        schema_version: DOCUMENT_SCHEMA_VERSION.to_owned(),
        content_json: imported.document_json,
        content_sha256,
        derived_json,
        search_text,
        entity: RecordId::new(ENTITIES, entity_id.clone()),
        entity_id: entity_id.clone(),
        bridge: RecordId::new(BRIDGES, document_id.clone()),
        placement: RecordId::new(PLACEMENTS, placement_id.clone()),
        placement_id,
        stage_provenance_key: card.stage_provenance_key.clone(),
        stage_provenance,
        provenance_sha256: card.stage_provenance.sha256.clone(),
        provenance_manifest_ref: card.stage_provenance.manifest_ref.clone(),
        provenance_correlation_id: card.stage_provenance.causal_action_id.clone(),
        x: card.x,
        y: card.y,
        w: card.w,
        h: card.h,
        z_index: i64::from(card.z_index),
        // RichDocument creation has no owner identity, so its same-id Loom
        // projection retains the legacy HUMAN/anonymous attribution.
        actor_id: None,
        actor_kind: "HUMAN".to_owned(),
        edit_event_id: document_metadata.edit_event_id.to_string(),
        written_at: Datetime::from(document_metadata.timestamp),
        detection_provenance: json!({
            "extractor": "loom_block_knowledge_bridge",
            "extractor_version": EXTRACTOR_VERSION,
            "method": "mt177_bridge",
            "content_type": "note",
        }),
        event,
    };

    // Result indexes: BEGIN=0; authority/board/key guards=1..3; document=4;
    // Loom block=5; search=6; version=7; entity=8; event=9; bridge=10;
    // placement=11; COMMIT=12.
    let rows: Vec<StagePlacementRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         IF (SELECT VALUE id FROM $artifact WHERE workspace_id = $workspace \
                           AND content_sha256 = $provenance_sha256 \
                           AND manifest_ref = $provenance_manifest_ref \
                           AND correlation_id = $provenance_correlation_id)[0] = NONE { \
                           THROW 'HSK-CANVAS-STAGE-AUTHORITY'; \
                         }; \
                         IF (SELECT VALUE id FROM $canvas WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-BOARD-NOT-FOUND'; \
                         }; \
                         IF array::len((SELECT id FROM loom_canvas_placements \
                           WHERE workspace_id = $workspace AND canvas_block_id = $canvas \
                             AND stage_provenance_key = $stage_provenance_key)) != 0 { \
                           THROW 'HSK-CANVAS-STAGE-PROVENANCE-CONFLICT'; \
                         }; \
                         CREATE $document CONTENT { rich_document_id: $document_id, \
                           workspace_id: $workspace, document_id: NONE, title: $document_title, \
                           schema_version: $schema_version, doc_version: 1, content_json: $content_json, \
                           content_sha256: $content_sha256, crdt_document_id: NONE, crdt_snapshot_id: NONE, \
                           promotion_receipt_event_id: NONE, projection_refs: [], project_ref: NONE, \
                           folder_ref: NONE, authority_label: 'promoted', owner_actor_kind: NONE, \
                           owner_actor_id: NONE, deleted_at: NONE, created_at: $written_at, updated_at: $written_at \
                         }; \
                         CREATE $block CONTENT { block_id: $document_id, workspace_id: $workspace, \
                           content_type: 'note', document_id: NONE, asset_id: NONE, title: $document_title, \
                           original_filename: NONE, content_hash: $content_sha256, pinned: false, favorite: false, \
                           pin_order: NONE, journal_date: NONE, last_job_id: NONE, last_workflow_id: NONE, \
                           last_actor_id: $actor_id, edit_event_id: $edit_event_id, last_actor_kind: $actor_kind, \
                           created_at: $written_at, updated_at: $written_at, imported_at: NONE, backlink_count: 0, \
                           mention_count: 0, tag_count: 0, derived_json: $derived_json, preview_status: 'none', \
                           thumbnail_asset_id: NONE, proxy_asset_id: NONE \
                         }; \
                         CREATE $search CONTENT { block_id: $block, workspace_id: $workspace, \
                           content_type: 'note', search_text: $search_text, embedding: NONE, \
                           embedding_model: NONE, indexed_at: $written_at \
                         }; \
                         CREATE knowledge_rich_document_versions CONTENT { rich_document_id: $document, \
                           doc_version: 1, schema_version: $schema_version, content_json: $content_json, \
                           content_sha256: $content_sha256, crdt_snapshot_id: NONE, \
                           promotion_receipt_event_id: NONE, created_at: $written_at \
                         }; \
                         CREATE $entity CONTENT { entity_id: $entity_id, workspace_id: $workspace, \
                           entity_kind: 'loom_block', entity_key: $document_id, display_name: $document_title, \
                           detection_provenance: $detection_provenance, lifecycle_state: 'active', \
                           primary_source_id: NONE, first_detected_in_run: NONE, last_detected_in_run: NONE, \
                           created_at: $written_at, updated_at: $written_at \
                         }; \
                         CREATE $event.record CONTENT { event_id: $event.event_id, \
                           event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, \
                           causation_id: $event.causation_id, correlation_id: $event.correlation_id, \
                           payload_hash: $event.payload_hash, source_component: $event.source_component, \
                           payload: $event.payload, created_at: $event.created_at \
                         }; \
                         CREATE $bridge CONTENT { block_id: $block, workspace_id: $workspace, \
                           entity_id: $entity, index_event_id: $event.record, \
                           created_at: $written_at, updated_at: $written_at \
                         }; \
                         CREATE $placement CONTENT { placement_id: $placement_id, canvas_block_id: $canvas, \
                           workspace_id: $workspace, placed_block_id: $block, x: $x, y: $y, w: $w, h: $h, \
                           z_index: $z_index, group_id: NONE, is_text_card: true, \
                           stage_provenance_key: $stage_provenance_key, stage_provenance: $stage_provenance, \
                           created_at: $written_at, updated_at: $written_at \
                         } RETURN AFTER; \
                         COMMIT TRANSACTION;",
                        bindings,
                        11,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let placement = rows.into_iter().next().ok_or_else(|| {
        StorageError::Database("Stage canvas placement create returned no row".to_owned())
    })?;
    let block = read_loom_block(storage, &card.workspace_id, &document_id).await?;
    Ok(LoomCanvasStageCard {
        block,
        rich_document_id: document_id,
        placement: stage_placement_to_domain(&placement)?,
        created_by_request: true,
    })
}

async fn read_stage_placement_by_id(
    storage: &SurrealStorage,
    workspace_id: &str,
    placement_id: &str,
) -> StorageResult<Option<StagePlacementRow>> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new(PLACEMENTS, placement_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, \
                           x, y, w, h, z_index, group_id, is_text_card, stage_provenance_key, \
                           stage_provenance, created_at, updated_at FROM $record \
                         WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn compensation_residue_exists(
    storage: &SurrealStorage,
    card: &CompensateLoomCanvasStageCard,
) -> StorageResult<bool> {
    let bindings = StageReceiptBindings {
        placement: RecordId::new(PLACEMENTS, card.placement_id.clone()),
        workspace: RecordId::new(WORKSPACES, card.workspace_id.clone()),
        canvas: RecordId::new(BOARDS, card.canvas_block_id.clone()),
        block: RecordId::new(BLOCKS, card.placed_block_id.clone()),
        document: RecordId::new(DOCUMENTS, card.placed_block_id.clone()),
        block_id: card.placed_block_id.clone(),
        expected_title: format!("Stage capture {}", card.stage_provenance.artifact_id),
        stage_provenance_key: card.stage_provenance_key.clone(),
    };
    let row: Option<PresenceRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT ( \
                           array::len((SELECT id FROM loom_canvas_placements \
                             WHERE workspace_id = $workspace AND canvas_block_id = $canvas \
                               AND stage_provenance_key = $stage_provenance_key)) > 0 \
                           OR array::len((SELECT id FROM loom_canvas_placements \
                             WHERE placed_block_id = $block)) > 0 \
                           OR array::len((SELECT id FROM knowledge_rich_documents \
                             WHERE id = $document)) > 0 \
                           OR array::len((SELECT id FROM loom_blocks WHERE id = $block)) > 0 \
                           OR array::len((SELECT id FROM loom_block_knowledge_bridge \
                             WHERE id = type::record('loom_block_knowledge_bridge', $block_id))) > 0 \
                           OR array::len((SELECT id FROM knowledge_entities \
                             WHERE workspace_id = $workspace AND entity_kind = 'loom_block' \
                               AND entity_key = $block_id)) > 0 \
                         ) AS present FROM [true] LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    Ok(row.is_some_and(|row| row.present))
}

async fn read_stage_document(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Option<StageDocumentRow>> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new(DOCUMENTS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT rich_document_id, workspace_id, document_id, title, schema_version, \
                           doc_version, content_json, content_sha256, crdt_document_id, crdt_snapshot_id, \
                           promotion_receipt_event_id, projection_refs, project_ref, folder_ref, \
                           authority_label, owner_actor_kind, owner_actor_id, deleted_at, created_at, updated_at \
                         FROM $record WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn read_stage_versions(
    storage: &SurrealStorage,
    block_id: &str,
) -> StorageResult<Vec<StageVersionRow>> {
    #[derive(SurrealValue)]
    struct Bindings {
        document: RecordId,
    }
    let bindings = Bindings {
        document: RecordId::new(DOCUMENTS, block_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT doc_version, schema_version, content_json, content_sha256, \
                           crdt_snapshot_id, promotion_receipt_event_id \
                         FROM knowledge_rich_document_versions WHERE rich_document_id = $document \
                         ORDER BY doc_version ASC;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn read_stage_block_ownership(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Option<StageBlockOwnershipRow>> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new(BLOCKS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT title, content_type, content_hash, document_id, asset_id, original_filename, \
                           pinned, favorite, pin_order, journal_date, last_job_id, last_workflow_id, \
                           last_actor_id, edit_event_id, last_actor_kind, imported_at, backlink_count, \
                           mention_count, tag_count, derived_json, preview_status, thumbnail_asset_id, \
                           proxy_asset_id, created_at, updated_at FROM $record WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn read_stage_bridge_ownership(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Option<StageBridgeOwnershipRow>> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new(BRIDGES, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT entity_id, index_event_id, created_at AS bridge_created_at, \
                           updated_at AS bridge_updated_at, entity_id.entity_kind AS entity_kind, \
                           entity_id.entity_key AS entity_key, entity_id.display_name AS display_name, \
                           entity_id.detection_provenance AS detection_provenance, \
                           entity_id.primary_source_id AS primary_source_id, \
                           entity_id.first_detected_in_run AS first_detected_in_run, \
                           entity_id.last_detected_in_run AS last_detected_in_run, \
                           entity_id.lifecycle_state AS lifecycle_state, \
                           entity_id.created_at AS entity_created_at, entity_id.updated_at AS entity_updated_at, \
                           index_event_id.event_type AS index_event_type, \
                           index_event_id.aggregate_type AS index_aggregate_type, \
                           index_event_id.aggregate_id AS index_aggregate_id, \
                           index_event_id.source_component AS index_source_component, \
                           index_event_id.payload AS index_payload FROM $record \
                         WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

async fn read_stage_search(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Option<SearchOwnershipRow>> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new("loom_block_search_index", block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_first(
                        "SELECT workspace_id, content_type, search_text, embedding, embedding_model \
                         FROM $record WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)
}

pub(crate) async fn compensate_stage_canvas_card(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    card: CompensateLoomCanvasStageCard,
) -> StorageResult<LoomCanvasStageCompensation> {
    use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
    use crate::knowledge_document::import::{import_snippet, ImportFormat};

    let stage_provenance =
        validated_stage_provenance(&card.stage_provenance_key, &card.stage_provenance)?;
    validate_write(storage, ctx, &card.placement_id).await?;
    validate_write(storage, ctx, &card.placed_block_id).await?;
    let _mutation_guard = CANVAS_MUTATION_LOCK.lock().await;

    let Some(placement) =
        read_stage_placement_by_id(storage, &card.workspace_id, &card.placement_id).await?
    else {
        if compensation_residue_exists(storage, &card).await? {
            return Err(StorageError::Validation(
                "Canvas Stage compensation receipt is absent but owned authority residue remains",
            ));
        }
        return Ok(LoomCanvasStageCompensation {
            removed_by_request: false,
        });
    };

    if record_key(placement.workspace_id.clone(), WORKSPACES)? != card.workspace_id
        || record_key(placement.canvas_block_id.clone(), BOARDS)? != card.canvas_block_id
        || record_key(placement.placed_block_id.clone(), BLOCKS)? != card.placed_block_id
        || !placement.is_text_card
        || placement.stage_provenance_key.as_deref() != Some(card.stage_provenance_key.as_str())
        || placement.stage_provenance.as_ref() != Some(&stage_provenance)
        || placement.created_at != placement.updated_at
    {
        return Err(StorageError::Validation(
            "Canvas Stage compensation receipt does not own the persisted placement tuple",
        ));
    }

    let expected_title = format!("Stage capture {}", card.stage_provenance.artifact_id);
    let expected_markdown = serde_json::to_string(&card.stage_provenance)?;
    let expected_document = import_snippet(&expected_markdown, ImportFormat::Markdown);
    let expected_sha = knowledge_canonical_json_sha256(&expected_document.document_json);
    let (expected_derived, expected_search_text) =
        rich_document_loom_projection(&expected_title, &expected_document.document_json)?;
    let expected_derived: Value = serde_json::from_str(&expected_derived)?;

    let document = read_stage_document(storage, &card.workspace_id, &card.placed_block_id)
        .await?
        .ok_or(StorageError::Validation(
            "Canvas Stage compensation RichDocument ownership tuple is incomplete",
        ))?;
    if document.rich_document_id != card.placed_block_id
        || record_key(document.workspace_id.clone(), WORKSPACES)? != card.workspace_id
        || document.document_id.is_some()
        || document.title != expected_title
        || document.schema_version != DOCUMENT_SCHEMA_VERSION
        || document.doc_version != 1
        || document.content_json != expected_document.document_json
        || document.content_sha256 != expected_sha
        || document.crdt_document_id.is_some()
        || document.crdt_snapshot_id.is_some()
        || document.promotion_receipt_event_id.is_some()
        || document.projection_refs != json!([])
        || document.project_ref.is_some()
        || document.folder_ref.is_some()
        || document.authority_label != "promoted"
        || document.owner_actor_kind.is_some()
        || document.owner_actor_id.is_some()
        || document.deleted_at.is_some()
        || document.created_at != document.updated_at
    {
        return Err(StorageError::Validation(
            "Canvas Stage compensation refuses a modified RichDocument",
        ));
    }

    let versions = read_stage_versions(storage, &card.placed_block_id).await?;
    if versions.len() != 1
        || versions[0].doc_version != 1
        || versions[0].schema_version != DOCUMENT_SCHEMA_VERSION
        || versions[0].content_json != expected_document.document_json
        || versions[0].content_sha256 != expected_sha
        || versions[0].crdt_snapshot_id.is_some()
        || versions[0].promotion_receipt_event_id.is_some()
    {
        return Err(StorageError::Validation(
            "Canvas Stage compensation refuses modified RichDocument version history",
        ));
    }

    let block = read_stage_block_ownership(storage, &card.workspace_id, &card.placed_block_id)
        .await?
        .ok_or(StorageError::Validation(
            "Canvas Stage compensation LoomBlock ownership tuple is incomplete",
        ))?;
    if block.title.as_deref() != Some(expected_title.as_str())
        || block.content_type != "note"
        || block.content_hash.as_deref() != Some(expected_sha.as_str())
        || block.document_id.is_some()
        || block.asset_id.is_some()
        || block.original_filename.is_some()
        || block.pinned
        || block.favorite
        || block.pin_order.is_some()
        || block.journal_date.is_some()
        || block.last_job_id.is_some()
        || block.last_workflow_id.is_some()
        || block.last_actor_id.is_some()
        || block.edit_event_id.trim().is_empty()
        || block.last_actor_kind != "HUMAN"
        || block.imported_at.is_some()
        || block.backlink_count != 0
        || block.mention_count != 0
        || block.tag_count != 0
        || block.derived_json != expected_derived
        || block.preview_status != "none"
        || block.thumbnail_asset_id.is_some()
        || block.proxy_asset_id.is_some()
        || block.created_at != block.updated_at
    {
        return Err(StorageError::Validation(
            "Canvas Stage compensation refuses a modified LoomBlock projection",
        ));
    }

    let bridge = read_stage_bridge_ownership(storage, &card.workspace_id, &card.placed_block_id)
        .await?
        .ok_or(StorageError::Validation(
            "Canvas Stage compensation knowledge bridge ownership tuple is incomplete",
        ))?;
    let entity_id = record_key(bridge.entity_id.clone(), ENTITIES)?;
    let index_event_id = record_key(bridge.index_event_id.clone(), EVENT_LEDGER)?;
    let expected_detection = json!({
        "extractor": "loom_block_knowledge_bridge",
        "extractor_version": EXTRACTOR_VERSION,
        "method": "mt177_bridge",
        "content_type": "note",
    });
    let expected_index_payload = json!({
        "type": "knowledge_loom_block_indexed",
        "workspace_id": card.workspace_id,
        "block_id": card.placed_block_id,
        "entity_id": entity_id,
        "content_type": "note",
        "extractor_version": EXTRACTOR_VERSION,
    });
    if bridge.entity_kind != "loom_block"
        || bridge.entity_key != card.placed_block_id
        || bridge.display_name != expected_title
        || bridge.detection_provenance != expected_detection
        || bridge.primary_source_id.is_some()
        || bridge.first_detected_in_run.is_some()
        || bridge.last_detected_in_run.is_some()
        || bridge.lifecycle_state != "active"
        || bridge.bridge_created_at != bridge.bridge_updated_at
        || bridge.entity_created_at != bridge.entity_updated_at
        || bridge.index_event_type != KernelEventType::KnowledgeLoomBlockIndexed.as_str()
        || bridge.index_aggregate_type != "knowledge_loom_block"
        || bridge.index_aggregate_id != entity_id
        || bridge.index_source_component != "loom_block_knowledge_bridge"
        || bridge.index_payload != expected_index_payload
    {
        return Err(StorageError::Validation(
            "Canvas Stage compensation refuses a modified knowledge projection",
        ));
    }

    let search = read_stage_search(storage, &card.workspace_id, &card.placed_block_id)
        .await?
        .ok_or(StorageError::Validation(
            "Canvas Stage compensation search projection ownership tuple is incomplete",
        ))?;
    if record_key(search.workspace_id, WORKSPACES)? != card.workspace_id
        || search.content_type != "note"
        || search.search_text != expected_search_text
        || search.embedding.is_some()
        || search.embedding_model.is_some()
    {
        return Err(StorageError::Validation(
            "Canvas Stage compensation refuses a modified search projection",
        ));
    }

    let run_id = format!("LOOM-STAGE-COMPENSATE-{}", card.placement_id);
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeRichDocumentDeleted,
        bridge_actor(ctx),
    )
    .aggregate("knowledge_rich_document", card.placed_block_id.clone())
    .idempotency_key(format!(
        "loom-stage-compensate:{}:{}:{}",
        card.workspace_id, card.placement_id, card.stage_provenance_key
    ))
    .causation_id(index_event_id.clone())
    .correlation_id(card.stage_provenance.causal_action_id.clone())
    .source_component("loom_canvas_stage_compensation")
    .payload(json!({
        "type": "knowledge_rich_document_deleted",
        "reason": "stage_canvas_card_compensation",
        "workspace_id": card.workspace_id,
        "canvas_block_id": card.canvas_block_id,
        "placement_id": card.placement_id,
        "block_id": card.placed_block_id,
        "rich_document_id": card.placed_block_id,
        "title": expected_title,
        "entity_id": entity_id,
        "artifact_id": card.stage_provenance.artifact_id,
        "sha256": card.stage_provenance.sha256,
        "manifest_ref": card.stage_provenance.manifest_ref,
        "causal_action_id": card.stage_provenance.causal_action_id,
        "stage_provenance_key": card.stage_provenance_key,
    }))
    .build()
    .map_err(|_| {
        StorageError::Validation("loom Stage compensation EventLedger receipt build failed")
    })?;
    let (_, event) = event_ledger::prepare_event(event)?;
    let bindings = StageCompensationBindings {
        placement: RecordId::new(PLACEMENTS, card.placement_id.clone()),
        workspace: RecordId::new(WORKSPACES, card.workspace_id.clone()),
        canvas: RecordId::new(BOARDS, card.canvas_block_id.clone()),
        block: RecordId::new(BLOCKS, card.placed_block_id.clone()),
        document: RecordId::new(DOCUMENTS, card.placed_block_id.clone()),
        search: RecordId::new("loom_block_search_index", card.placed_block_id.clone()),
        bridge: RecordId::new(BRIDGES, card.placed_block_id.clone()),
        entity: RecordId::new(ENTITIES, entity_id.clone()),
        entity_id,
        index_event: RecordId::new(EVENT_LEDGER, index_event_id),
        block_id: card.placed_block_id,
        expected_title,
        schema_version: DOCUMENT_SCHEMA_VERSION.to_owned(),
        content_json: expected_document.document_json,
        content_sha256: expected_sha,
        derived_json: expected_derived,
        search_text: expected_search_text,
        stage_provenance_key: card.stage_provenance_key,
        stage_provenance,
        detection_provenance: expected_detection,
        index_payload: expected_index_payload,
        event,
    };

    // Result indexes: BEGIN=0; six ownership/reference guards=1..6;
    // compensation event=7; five exact deletes=8..12; COMMIT=13.
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<surrealdb::types::Value, _>(
                        "BEGIN TRANSACTION; \
                         IF array::len((SELECT id FROM $placement WHERE workspace_id = $workspace \
                           AND canvas_block_id = $canvas AND placed_block_id = $block \
                           AND is_text_card = true AND stage_provenance_key = $stage_provenance_key \
                           AND stage_provenance = $stage_provenance AND created_at = updated_at)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-PLACEMENT'; \
                         }; \
                         IF array::len((SELECT id FROM $document WHERE workspace_id = $workspace \
                           AND rich_document_id = $block_id AND document_id = NONE \
                           AND title = $expected_title AND schema_version = $schema_version \
                           AND doc_version = 1 AND content_json = $content_json \
                           AND content_sha256 = $content_sha256 AND crdt_document_id = NONE \
                           AND crdt_snapshot_id = NONE AND promotion_receipt_event_id = NONE \
                           AND projection_refs = [] AND project_ref = NONE AND folder_ref = NONE \
                           AND authority_label = 'promoted' AND owner_actor_kind = NONE \
                           AND owner_actor_id = NONE AND deleted_at = NONE AND created_at = updated_at)) != 1 \
                           OR array::len((SELECT id FROM knowledge_rich_document_versions \
                             WHERE rich_document_id = $document AND doc_version = 1 \
                               AND schema_version = $schema_version AND content_json = $content_json \
                               AND content_sha256 = $content_sha256 AND crdt_snapshot_id = NONE \
                               AND promotion_receipt_event_id = NONE)) != 1 \
                           OR array::len((SELECT id FROM knowledge_rich_document_versions \
                             WHERE rich_document_id = $document)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DOCUMENT'; \
                         }; \
                         IF array::len((SELECT id FROM $block WHERE workspace_id = $workspace \
                           AND title = $expected_title AND content_type = 'note' \
                           AND content_hash = $content_sha256 AND document_id = NONE AND asset_id = NONE \
                           AND original_filename = NONE AND pinned = false AND favorite = false \
                           AND pin_order = NONE AND journal_date = NONE AND last_job_id = NONE \
                           AND last_workflow_id = NONE AND last_actor_id = NONE \
                           AND edit_event_id != '' AND last_actor_kind = 'HUMAN' AND imported_at = NONE \
                           AND backlink_count = 0 AND mention_count = 0 AND tag_count = 0 \
                           AND derived_json = $derived_json AND preview_status = 'none' \
                           AND thumbnail_asset_id = NONE AND proxy_asset_id = NONE \
                           AND created_at = updated_at)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-BLOCK'; \
                         }; \
                         IF array::len((SELECT id FROM $bridge WHERE workspace_id = $workspace \
                           AND block_id = $block AND entity_id = $entity \
                           AND index_event_id = $index_event AND created_at = updated_at)) != 1 \
                           OR array::len((SELECT id FROM $entity WHERE workspace_id = $workspace \
                             AND entity_kind = 'loom_block' AND entity_key = $block_id \
                             AND display_name = $expected_title \
                             AND detection_provenance = $detection_provenance \
                             AND primary_source_id = NONE AND first_detected_in_run = NONE \
                             AND last_detected_in_run = NONE AND lifecycle_state = 'active' \
                             AND created_at = updated_at)) != 1 \
                           OR array::len((SELECT id FROM $index_event \
                             WHERE event_type = 'KNOWLEDGE_LOOM_BLOCK_INDEXED' \
                               AND aggregate_type = 'knowledge_loom_block' \
                               AND aggregate_id = $entity_id \
                               AND source_component = 'loom_block_knowledge_bridge' \
                               AND payload = $index_payload)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-BRIDGE'; \
                         }; \
                         IF array::len((SELECT id FROM $search WHERE workspace_id = $workspace \
                           AND content_type = 'note' AND search_text = $search_text \
                           AND embedding = NONE AND embedding_model = NONE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-SEARCH'; \
                         }; \
                         IF array::len((SELECT id FROM loom_canvas_placements \
                           WHERE placed_block_id = $block AND id != $placement)) > 0 \
                           OR array::len((SELECT id FROM loom_canvas_visual_edges \
                             WHERE from_placement_id = $placement OR to_placement_id = $placement)) > 0 \
                           OR array::len((SELECT id FROM loom_edges WHERE workspace_id = $workspace \
                             AND (source_block_id = $block OR target_block_id = $block \
                               OR source_text_block_id = $block_id))) > 0 \
                           OR array::len((SELECT id FROM knowledge_sources WHERE loom_block_id = $block \
                             OR (workspace_id = $workspace AND source_kind = 'rich_document' \
                               AND provenance.rich_document_id = $block_id))) > 0 \
                           OR array::len((SELECT id FROM loom_folder_members WHERE block_id = $block)) > 0 \
                           OR array::len((SELECT id FROM loom_canvas_boards WHERE block_id = $block)) > 0 \
                           OR array::len((SELECT id FROM atelier_intake_item_loom_projection \
                             WHERE loom_block_id = $block)) > 0 \
                           OR array::len((SELECT id FROM knowledge_edges \
                             WHERE source_entity_id = $entity OR target_entity_id = $entity)) > 0 \
                           OR array::len((SELECT id FROM knowledge_entity_spans WHERE entity_id = $entity)) > 0 \
                           OR array::len((SELECT id FROM knowledge_claims WHERE subject_entity_id = $entity)) > 0 \
                           OR array::len((SELECT id FROM knowledge_code_files WHERE file_entity_id = $entity)) > 0 \
                           OR array::len((SELECT id FROM knowledge_memory_facts \
                             WHERE subject_entity_id = $entity OR object_entity_id = $entity)) > 0 \
                           OR array::len((SELECT id FROM knowledge_memory_bridge_decisions \
                             WHERE entity_id_a = $entity OR entity_id_b = $entity)) > 0 \
                           OR array::len((SELECT id FROM knowledge_rich_document_drafts \
                             WHERE rich_document_id = $document)) > 0 \
                           OR array::len((SELECT id FROM knowledge_editor_code_nodes \
                             WHERE rich_document_id = $document)) > 0 \
                           OR array::len((SELECT id FROM knowledge_document_embeds \
                             WHERE rich_document_id = $document)) > 0 \
                           OR array::len((SELECT id FROM knowledge_document_backlinks \
                             WHERE workspace_id = $workspace AND (source_document_id = $document \
                               OR target = $block_id OR target = $expected_title))) > 0 \
                           OR array::len((SELECT id FROM knowledge_debug_breakpoints \
                             WHERE rich_document_id = $document)) > 0 \
                           OR array::len((SELECT id FROM knowledge_context_bundle_items \
                             WHERE ref_kind = 'entity' AND ref_id = $entity_id \
                               AND bundle_id.workspace_id = $workspace)) > 0 \
                           OR array::len((SELECT id FROM fems_memory_proposals \
                             WHERE workspace_id = $workspace AND document_id = $block_id)) > 0 \
                           OR array::len((SELECT id FROM loom_ai_suggestions \
                             WHERE workspace_id = $workspace \
                               AND (block_id = $block_id OR target_block_id = $block_id))) > 0 \
                           OR array::len((SELECT id FROM knowledge_quick_switcher_recents \
                             WHERE workspace_id = $workspace AND ((source_kind = 'loom_block' \
                               AND ref_id = $block_id) OR (result_kind = 'knowledge_entity' \
                               AND ref_id = $entity_id)))) > 0 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-REFERENCES'; \
                         }; \
                         CREATE $event.record CONTENT { event_id: $event.event_id, \
                           event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, \
                           causation_id: $event.causation_id, correlation_id: $event.correlation_id, \
                           payload_hash: $event.payload_hash, source_component: $event.source_component, \
                           payload: $event.payload, created_at: $event.created_at \
                         }; \
                         IF array::len((DELETE $placement RETURN BEFORE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DELETE'; \
                         }; \
                         IF array::len((DELETE $bridge RETURN BEFORE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DELETE'; \
                         }; \
                         IF array::len((DELETE $entity RETURN BEFORE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DELETE'; \
                         }; \
                         IF array::len((DELETE $document RETURN BEFORE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DELETE'; \
                         }; \
                         IF array::len((DELETE $block RETURN BEFORE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DELETE'; \
                         }; \
                         COMMIT TRANSACTION;",
                        bindings,
                        7,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;

    Ok(LoomCanvasStageCompensation {
        removed_by_request: true,
    })
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
                           x, y, w, h, z_index, group_id, is_text_card, created_at, updated_at \
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
                           x, y, w, h, z_index, group_id, is_text_card, created_at, updated_at \
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
    let count = storage
        .with_data_operation({
            let bindings = RecordWorkspaceBindings {
                record: RecordId::new(PLACEMENTS, placement_id.to_owned()),
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
        Err(StorageError::NotFound("loom_canvas_placement"))
    }
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
