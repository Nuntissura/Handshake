//! Embedded SurrealDB persistence for Loom CanvasBoard.
//!
//! Canvas placements are references to Loom blocks, never content copies. The
//! Stage-card path is deliberately implemented here as one transaction because
//! its RichDocument, Loom/search projection, knowledge bridge, placement, and
//! EventLedger receipt form one compensation-owned authority tuple.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use uuid::Uuid;

#[cfg(any(test, feature = "surreal-test-support"))]
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock, Mutex as StdMutex,
    },
};
#[cfg(any(test, feature = "surreal-test-support"))]
use tokio::sync::Notify;

use super::keyed_lock::LockKey;
use super::retry::Replay;
use super::{
    current_record_user_scope, event_ledger, loom_store, SurrealDataContext, SurrealDatabase,
    SurrealStorage, SurrealStorageError,
};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{knowledge_canonical_json_sha256, rich_document_loom_projection};
use crate::storage::{
    CompensateLoomCanvasStageCard, LoomBlock, LoomBlockContentType, LoomCanvasBoard,
    LoomCanvasBoardView, LoomCanvasPlacement, LoomCanvasPlacementCreateReceipt,
    LoomCanvasPlacementRemovalReceipt, LoomCanvasPlacementUpdate, LoomCanvasStageCard,
    LoomCanvasStageCompensation, LoomCanvasStageProvenance, LoomCanvasVisualEdge,
    LoomMutationEventReceipt, MutationMetadata, NewLoomBlock, NewLoomCanvasPlacement,
    NewLoomCanvasStageCard, StorageError, StorageResult, WriteActorKind, WriteContext,
    LOOM_CANVAS_BOARD_SCHEMA_ID, LOOM_CANVAS_STAGE_PROVENANCE_SCHEMA,
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
const OWNED_LOOM_BUNDLE_SQL: &str = "BEGIN TRANSACTION; \
    IF $creator != $auth.id OR !fn::mt109_live_session() OR $authorizing_grant = NONE { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF record::exists($block) { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $block CONTENT $content RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $owned_resource SET resource_kind = 'loom_block', external_resource_id = record::id($block), owner_account_id = $creator.account_id, created_by_principal_id = $creator.principal_id, created_in_session_id = $creator, creator_grant_id = $owned_grant, access_space_id = $creator.access_space_id, parent_resource_id = $parent, schema_version = 1, lifecycle_state = 'active', policy_version = $creator.policy_version, classification = 'account_private', storage_locator_hash = $locator_hash, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $owned_grant SET account_id = $creator.account_id, principal_id = $creator.principal_id, access_space_id = $creator.access_space_id, resource_id = $owned_resource, actions = ['read','create','update','delete'], capability_ids = ['fs.read','fs.write'], delegation_chain = $creator.delegation_chain, status = 'active', grant_version = 1, policy_version = $creator.policy_version, expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((UPSERT $search SET block_id = $block, workspace_id = $workspace, content_type = $content_type, search_text = $search_text, indexed_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $bridge_event.record CONTENT { event_id: $bridge_event.event_id, event_version: $bridge_event.event_version, kernel_task_run_id: $bridge_event.kernel_task_run_id, session_run_id: $bridge_event.session_run_id, aggregate_type: $bridge_event.aggregate_type, aggregate_id: $bridge_event.aggregate_id, idempotency_key: $bridge_event.idempotency_key, event_type: $bridge_event.event_type, actor_kind: $bridge_event.actor_kind, actor_id: $bridge_event.actor_id, causation_id: $bridge_event.causation_id, correlation_id: $bridge_event.correlation_id, payload_hash: $bridge_event.payload_hash, source_component: $bridge_event.source_component, payload: $bridge_event.payload, wsids: $bridge_event.wsids, authority_resource_id: $bridge_event.authority_resource_id, authority_session_id: $bridge_event.authority_session_id, authority_capability_id: $bridge_event.authority_capability_id, authority_action: $bridge_event.authority_action, created_at: $bridge_event.created_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $entity SET entity_id = $entity_id, workspace_id = $workspace, entity_kind = 'loom_block', entity_key = record::id($block), display_name = $display_name, detection_provenance = $detection_provenance, lifecycle_state = 'active', updated_at = $updated_at RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $bridge SET block_id = $block, workspace_id = $workspace, entity_id = $entity, index_event_id = $bridge_event.record, updated_at = $updated_at RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF $board != NONE { IF array::len((CREATE $board_event.record CONTENT { event_id: $board_event.event_id, event_version: $board_event.event_version, kernel_task_run_id: $board_event.kernel_task_run_id, session_run_id: $board_event.session_run_id, aggregate_type: $board_event.aggregate_type, aggregate_id: $board_event.aggregate_id, idempotency_key: $board_event.idempotency_key, event_type: $board_event.event_type, actor_kind: $board_event.actor_kind, actor_id: $board_event.actor_id, causation_id: $board_event.causation_id, correlation_id: $board_event.correlation_id, payload_hash: $board_event.payload_hash, source_component: $board_event.source_component, payload: $board_event.payload, wsids: $board_event.wsids, authority_resource_id: $board_event.authority_resource_id, authority_session_id: $board_event.authority_session_id, authority_capability_id: $board_event.authority_capability_id, authority_action: $board_event.authority_action, created_at: $board_event.created_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; IF array::len((CREATE $board SET block_id = $block, workspace_id = $workspace, board_state = $board_state, event_ledger_event_id = $board_event.record, updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; }; \
    LET $creator_account = $creator.account_id; LET $creator_principal = $creator.principal_id; LET $creator_space = $creator.access_space_id; \
    IF array::len((UPDATE $creator SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $parent SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $authorizing_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((SELECT VALUE id FROM $block WHERE workspace_id = $workspace AND source_rich_document_id = NONE AND created_in_session_id = $creator)) != 1 OR array::len((SELECT VALUE id FROM $owned_resource WHERE resource_kind = 'loom_block' AND owner_account_id = $creator.account_id AND created_by_principal_id = $creator.principal_id AND created_in_session_id = $creator AND creator_grant_id = $owned_grant AND access_space_id = $creator.access_space_id AND parent_resource_id = $parent AND lifecycle_state = 'active')) != 1 OR array::len((SELECT VALUE id FROM $owned_grant WHERE account_id = $creator.account_id AND principal_id = $creator.principal_id AND access_space_id = $creator.access_space_id AND resource_id = $owned_resource AND actions = ['read','create','update','delete'] AND capability_ids = ['fs.read','fs.write'] AND delegation_chain = $creator.delegation_chain AND status = 'active')) != 1 OR array::len((SELECT VALUE id FROM $search WHERE block_id = $block AND workspace_id = $workspace)) != 1 OR array::len((SELECT VALUE id FROM $entity WHERE workspace_id = $workspace AND entity_kind = 'loom_block' AND entity_key = record::id($block))) != 1 OR array::len((SELECT VALUE id FROM $bridge_event.record)) != 1 OR array::len((SELECT VALUE id FROM $bridge WHERE block_id = $block AND workspace_id = $workspace AND entity_id = $entity AND index_event_id = $bridge_event.record)) != 1 OR ($board != NONE AND (array::len((SELECT VALUE id FROM $board_event.record)) != 1 OR array::len((SELECT VALUE id FROM $board WHERE block_id = $block AND workspace_id = $workspace AND event_ledger_event_id = $board_event.record)) != 1)) { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    COMMIT TRANSACTION;";

#[cfg(test)]
async fn execute_record_user_loom_bundle_with_test_diagnostics(
    db: &SurrealDataContext<'_>,
    bindings: OwnedLoomBundleBindings,
) -> Result<usize, SurrealStorageError> {
    let mut response = db
        .client
        .query(OWNED_LOOM_BUNDLE_SQL)
        .bind(SurrealValue::into_value(bindings))
        .await?;
    let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
    errors.sort_by_key(|(statement_index, _)| *statement_index);
    if !errors.is_empty() {
        let meaningful = errors
            .iter()
            .position(|(_, error)| {
                !error
                    .to_string()
                    .to_ascii_lowercase()
                    .contains("query was not executed due to a failed transaction")
            })
            .unwrap_or(0);
        let (statement_index, error) = errors.swap_remove(meaningful);
        eprintln!(
            "record-user-loom-bundle transactionfailed statement_index={statement_index} error={error}"
        );
        return Err(error.into());
    }
    let rows: Vec<surrealdb::types::Value> = response.take(0)?;
    Ok(rows.len())
}

// Concurrency (MT-152 I-152-2, replacing the process-global Canvas mutation
// mutex): every Canvas invariant is owned by the guards inside its one
// transaction (board identity and stale-viewport THROWs, `uq_loom_canvas_placement`,
// `idx_loom_canvas_stage_provenance`, the compensation ownership/reference
// THROWs) and by `pk_*` on each row. Each mutation below runs through
// `SurrealDatabase::guarded_mutation` on the narrowest stable key: the board
// row for board writes, the placement row for placement writes, the placed
// Loom block for a new placement (the row a concurrent Stage compensation
// deletes, so a writer queued behind a compensation revalidates the deleted
// block in its own transaction), the (workspace, canvas, provenance key)
// natural key for a Stage card, and both endpoint placements, in sorted order,
// for a visual edge.

#[cfg(any(test, feature = "surreal-test-support"))]
struct StageCompensationBarrierState {
    entered: AtomicBool,
    writer_waiting: AtomicBool,
    released: AtomicBool,
    changed: Notify,
}

#[cfg(any(test, feature = "surreal-test-support"))]
static STAGE_COMPENSATION_BARRIERS: LazyLock<
    StdMutex<HashMap<String, Arc<StageCompensationBarrierState>>>,
> = LazyLock::new(|| StdMutex::new(HashMap::new()));

#[cfg(any(test, feature = "surreal-test-support"))]
impl SurrealStorage {
    /// Arms a deterministic pause after compensation has validated ownership
    /// and references while it still holds the placed block's keyed lock.
    pub fn test_arm_stage_compensation_barrier(&self, placed_block_id: &str) {
        STAGE_COMPENSATION_BARRIERS
            .lock()
            .expect("stage compensation barrier registry poisoned")
            .insert(
                placed_block_id.to_owned(),
                Arc::new(StageCompensationBarrierState {
                    entered: AtomicBool::new(false),
                    writer_waiting: AtomicBool::new(false),
                    released: AtomicBool::new(false),
                    changed: Notify::new(),
                }),
            );
    }

    pub async fn test_wait_for_stage_compensation_barrier(&self, placed_block_id: &str) {
        let state = STAGE_COMPENSATION_BARRIERS
            .lock()
            .expect("stage compensation barrier registry poisoned")
            .get(placed_block_id)
            .cloned()
            .expect("stage compensation barrier was not armed");
        loop {
            let changed = state.changed.notified();
            if state.entered.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    pub fn test_release_stage_compensation_barrier(&self, placed_block_id: &str) {
        if let Some(state) = STAGE_COMPENSATION_BARRIERS
            .lock()
            .expect("stage compensation barrier registry poisoned")
            .get(placed_block_id)
            .cloned()
        {
            state.released.store(true, Ordering::Release);
            state.changed.notify_waiters();
        }
    }

    pub async fn test_wait_for_stage_reference_writer(&self, placed_block_id: &str) {
        let state = STAGE_COMPENSATION_BARRIERS
            .lock()
            .expect("stage compensation barrier registry poisoned")
            .get(placed_block_id)
            .cloned()
            .expect("stage compensation barrier was not armed");
        loop {
            let changed = state.changed.notified();
            if state.writer_waiting.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    pub fn test_reset_stage_compensation_barrier(&self, placed_block_id: &str) {
        self.test_release_stage_compensation_barrier(placed_block_id);
        STAGE_COMPENSATION_BARRIERS
            .lock()
            .expect("stage compensation barrier registry poisoned")
            .remove(placed_block_id);
    }

    /// Late failure inside the production compensation transaction. The block
    /// delete is last, after the audit append and four earlier deletes.
    pub async fn test_set_stage_compensation_delete_failpoint(
        &self,
        enabled: bool,
    ) -> StorageResult<()> {
        let statement = if enabled {
            "DEFINE EVENT OVERWRITE mt141_stage_compensation_delete_failpoint \
             ON TABLE loom_blocks WHEN $event = 'DELETE' \
             THEN { THROW 'MT141-STAGE-COMPENSATION-DELETE'; };"
        } else {
            "REMOVE EVENT mt141_stage_compensation_delete_failpoint ON TABLE loom_blocks;"
        };
        self.with_data_operation(move |database| {
            Box::pin(async move {
                database.client.query(statement).await?.check()?;
                Ok(())
            })
        })
        .await
        .map_err(map_err)
    }

    /// Attempts a typed persisted Stage provenance replacement through the
    /// real schema boundary. Invalid objects must be rejected by the current
    /// SCHEMAFULL fields or `enforce_loom_canvas_stage_provenance` event.
    pub async fn test_try_set_stage_provenance_json(
        &self,
        placement_id: &str,
        stage_provenance: Value,
    ) -> StorageResult<()> {
        let bindings = StageProvenanceJsonTestBindings {
            placement: RecordId::new(PLACEMENTS, placement_id.to_owned()),
            stage_provenance,
        };
        let rows: Vec<RecordId> = self
            .with_data_operation(move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "UPDATE $placement SET stage_provenance = $stage_provenance \
                             RETURN VALUE id;",
                            bindings,
                        )
                        .await
                })
            })
            .await
            .map_err(map_err)?;
        if rows.len() != 1 {
            return Err(StorageError::NotFound("loom_canvas_placement"));
        }
        Ok(())
    }

    pub async fn test_try_clear_stage_provenance(&self, placement_id: &str) -> StorageResult<()> {
        self.test_try_clear_stage_provenance_field(
            placement_id,
            "UPDATE $placement SET stage_provenance = NONE RETURN VALUE id;",
        )
        .await
    }

    pub async fn test_try_clear_stage_provenance_key(
        &self,
        placement_id: &str,
    ) -> StorageResult<()> {
        self.test_try_clear_stage_provenance_field(
            placement_id,
            "UPDATE $placement SET stage_provenance_key = NONE RETURN VALUE id;",
        )
        .await
    }

    async fn test_try_clear_stage_provenance_field(
        &self,
        placement_id: &str,
        statement: &'static str,
    ) -> StorageResult<()> {
        let bindings = StageProvenanceRecordTestBindings {
            placement: RecordId::new(PLACEMENTS, placement_id.to_owned()),
        };
        let rows: Vec<RecordId> = self
            .with_data_operation(move |database| {
                Box::pin(async move { database.query_values(statement, bindings).await })
            })
            .await
            .map_err(map_err)?;
        if rows.len() != 1 {
            return Err(StorageError::NotFound("loom_canvas_placement"));
        }
        Ok(())
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
async fn pause_stage_compensation_after_validation(placed_block_id: &str) {
    let state = STAGE_COMPENSATION_BARRIERS
        .lock()
        .expect("stage compensation barrier registry poisoned")
        .get(placed_block_id)
        .cloned();
    let Some(state) = state else {
        return;
    };
    state.entered.store(true, Ordering::Release);
    state.changed.notify_waiters();
    loop {
        let changed = state.changed.notified();
        if state.released.load(Ordering::Acquire) {
            break;
        }
        changed.await;
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
fn mark_stage_reference_writer_waiting(placed_block_id: &str) {
    if let Some(state) = STAGE_COMPENSATION_BARRIERS
        .lock()
        .expect("stage compensation barrier registry poisoned")
        .get(placed_block_id)
        .cloned()
    {
        state.writer_waiting.store(true, Ordering::Release);
        state.changed.notify_waiters();
    }
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct StageProvenanceJsonTestBindings {
    placement: RecordId,
    stage_provenance: Value,
}

#[cfg(any(test, feature = "surreal-test-support"))]
#[derive(SurrealValue)]
struct StageProvenanceRecordTestBindings {
    placement: RecordId,
}

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

#[derive(Clone, SurrealValue)]
struct OwnedLoomBlockContent {
    block_id: String,
    workspace_id: RecordId,
    created_in_session_id: RecordId,
    content_type: String,
    document_id: Option<RecordId>,
    asset_id: Option<RecordId>,
    title: Option<String>,
    original_filename: Option<String>,
    content_hash: Option<String>,
    pinned: bool,
    favorite: bool,
    pin_order: Option<i64>,
    journal_date: Option<String>,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    edit_event_id: String,
    last_actor_kind: String,
    created_at: Datetime,
    updated_at: Datetime,
    imported_at: Option<Datetime>,
    backlink_count: i64,
    mention_count: i64,
    tag_count: i64,
    derived_json: Value,
    preview_status: String,
    thumbnail_asset_id: Option<RecordId>,
    proxy_asset_id: Option<RecordId>,
}

#[derive(Clone, SurrealValue)]
struct OwnedLoomBundleBindings {
    block: RecordId,
    content: OwnedLoomBlockContent,
    search: RecordId,
    workspace: RecordId,
    content_type: String,
    search_text: String,
    creator: RecordId,
    parent: RecordId,
    authorizing_grant: Option<RecordId>,
    owned_resource: RecordId,
    owned_grant: RecordId,
    locator_hash: String,
    entity: RecordId,
    entity_id: String,
    display_name: String,
    detection_provenance: Value,
    bridge: RecordId,
    bridge_event: event_ledger::LedgerWrite,
    board_event: Option<event_ledger::LedgerWrite>,
    board: Option<RecordId>,
    board_state: Option<Value>,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct MutationEventRow {
    event_id: String,
    event_sequence: i64,
    created_at: Datetime,
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
struct RecordUserPlacementBindings {
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
    creator: RecordId,
    board_resource: RecordId,
    source_resource: RecordId,
    board_grant: RecordId,
    source_grant: RecordId,
    event: event_ledger::LedgerWrite,
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
struct RecordUserPlacementRemovalBindings {
    placement: RecordId,
    workspace: RecordId,
    canvas: RecordId,
    placed_block: RecordId,
    creator: RecordId,
    board_resource: RecordId,
    source_resource: RecordId,
    board_grant: RecordId,
    source_grant: RecordId,
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
    is_text_card: bool,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct PlacementCreateReceiptRow {
    placement: PlacementRow,
    event: MutationEventRow,
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
    // MT-153 AC-153-7: record-user authority witnesses (NONE on the root/storage-proof path).
    creator: Option<RecordId>,
    parent: Option<RecordId>,
    authorizing_grant: Option<RecordId>,
    owned_resource: Option<RecordId>,
    owned_grant: Option<RecordId>,
    locator_hash: Option<String>,
}

/// MT-153 AC-153-7 (Master Spec 02-system-architecture.md:2773/2776): the record-user witnesses of a
/// Stage card created inside the account's workspace `fs.write`/Create scope. The RichDocument gets its
/// `rich_document` protected resource and creator grant exactly as `create_owned_document_rows` (the
/// non-Stage text-card path) mints them; `None` when no record-user scope is active (root proofs).
struct StageRecordUserWitness {
    creator: RecordId,
    parent: RecordId,
    authorizing_grant: RecordId,
    owned_resource: RecordId,
    owned_grant: RecordId,
    locator_hash: String,
}

fn stage_record_user_witness(
    workspace_id: &str,
    document_id: &str,
) -> StorageResult<Option<StageRecordUserWitness>> {
    let Some(scope) = current_record_user_scope() else {
        return Ok(None);
    };
    if scope.workspace_id.as_deref() != Some(workspace_id)
        || scope.capability_id != "fs.write"
        || !matches!(
            scope.action,
            super::resource_authority::ResourceAction::Create
        )
    {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    let grant_id = scope
        .grant_id
        .clone()
        .ok_or(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"))?;
    Ok(Some(StageRecordUserWitness {
        creator: RecordId::new("authenticated_sessions", scope.session_id.clone()),
        parent: RecordId::new("protected_resources", scope.resource_id.clone()),
        authorizing_grant: RecordId::new("resource_grants", grant_id),
        owned_resource: RecordId::new("protected_resources", Uuid::now_v7().to_string()),
        owned_grant: RecordId::new("resource_grants", Uuid::now_v7().to_string()),
        locator_hash: hex::encode(Sha256::digest(
            format!("rich_document:{document_id}").as_bytes(),
        )),
    }))
}

/// The session of a record-user Stage compensation, which must run in the account's workspace
/// `fs.write`/Delete scope (LM-RLS-001: a hard delete is a delete-grant action); `None` on the root path.
fn stage_compensation_creator(workspace_id: &str) -> StorageResult<Option<RecordId>> {
    let Some(scope) = current_record_user_scope() else {
        return Ok(None);
    };
    if scope.workspace_id.as_deref() != Some(workspace_id)
        || scope.capability_id != "fs.write"
        || !matches!(
            scope.action,
            super::resource_authority::ResourceAction::Delete
        )
        || scope.grant_id.is_none()
    {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    Ok(Some(RecordId::new(
        "authenticated_sessions",
        scope.session_id,
    )))
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
    /// MT-153 AC-153-7: the compensating session on the record-user path (NONE on the root path).
    creator: Option<RecordId>,
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
    } else if rendered.contains("HSK-403-PROTECTED-RESOURCE") {
        StorageError::Guard("HSK-403-PROTECTED-RESOURCE")
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

fn bridge_actor_from_metadata(metadata: &MutationMetadata) -> KernelActor {
    let actor_id = metadata
        .actor_id
        .clone()
        .unwrap_or_else(|| "loom_block_knowledge_bridge".to_owned());
    match metadata.actor_kind {
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

fn prepare_record_user_placement_event(
    metadata: &MutationMetadata,
    placement_id: &str,
    workspace_id: &str,
    canvas_block_id: &str,
    placed_block_id: &str,
    payload_type: &'static str,
    operation: &'static str,
) -> StorageResult<event_ledger::LedgerWrite> {
    let run_id = format!("LOOM-CANVAS-PLACEMENT-{placement_id}");
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomCanvasBoardRecorded,
        bridge_actor_from_metadata(metadata),
    )
    .aggregate("loom_canvas_placement", placement_id.to_owned())
    .source_component("loom_canvas_board")
    .idempotency_key(format!(
        "loom-canvas-placement:{operation}:{placement_id}:{}",
        metadata.edit_event_id
    ))
    .payload(json!({
        "type": payload_type,
        "op": operation,
        "workspace_id": workspace_id,
        "canvas_block_id": canvas_block_id,
        "placement_id": placement_id,
        "placed_block_id": placed_block_id,
    }))
    .build()
    .map_err(|_| StorageError::Validation("record-user Canvas placement event build failed"))?;
    let (_, mut write) = event_ledger::prepare_event(event)?;
    // The task-local scope remains the exact canvas resource witness; the
    // receipt carries the independently bound canonical workspace witness.
    write.wsids = vec![workspace_id.to_owned()];
    Ok(write)
}

fn require_record_user_placement_scopes(
    _workspace_id: &str,
    canvas_block_id: &str,
    _placed_block_id: &str,
    source_scope: &super::resource_authority::RecordUserScope,
) -> StorageResult<()> {
    let board_scope =
        current_record_user_scope().ok_or(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"))?;
    if board_scope.workspace_id.as_deref() != Some(canvas_block_id)
        || board_scope.capability_id != "fs.write"
        || !matches!(
            board_scope.action,
            super::resource_authority::ResourceAction::Update
        )
        || source_scope.capability_id != "fs.read"
        || !matches!(
            source_scope.action,
            super::resource_authority::ResourceAction::Read
        )
        || source_scope.session_id != board_scope.session_id
        || source_scope.session_token != board_scope.session_token
        || source_scope.channel_binding_hash != board_scope.channel_binding_hash
        || board_scope.grant_id.is_none()
        || source_scope.grant_id.is_none()
    {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    Ok(())
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
    database: &SurrealDatabase,
    ctx: &WriteContext,
    workspace_id: &str,
    block_id: &str,
    board_state: Value,
) -> StorageResult<LoomCanvasBoard> {
    validate_board_state(&board_state)?;
    validate_write(database.storage(), ctx, block_id).await?;
    let board_state = &board_state;
    database
        .guarded_mutation(
            vec![LockKey::record(BOARDS, block_id.to_owned())],
            Replay::idempotent(format!("canvas-board-create:{block_id}")),
            None,
            || {
                create_canvas_board_attempt(
                    database.storage(),
                    workspace_id,
                    block_id,
                    board_state.clone(),
                )
            },
        )
        .await
}

/// The authenticated native-editor create path. The Loom block, its searchable
/// projection, child authority, knowledge bridge, receipts, and optional board
/// must commit together so a record-user request never leaves a source-free
/// block behind.
pub(crate) async fn create_record_user_loom_bundle(
    db: &SurrealDataContext<'_>,
    block: NewLoomBlock,
    board_state: Option<Value>,
    metadata: MutationMetadata,
    workspace_id: &str,
) -> StorageResult<LoomBlock> {
    let scope =
        current_record_user_scope().ok_or(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"))?;
    if scope.workspace_id.as_deref() != Some(workspace_id)
        || scope.capability_id != "fs.write"
        || !matches!(
            scope.action,
            crate::storage::surreal::resource_authority::ResourceAction::Create
        )
    {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    if block.workspace_id != workspace_id {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    let block_id = block
        .block_id
        .clone()
        .filter(|id| !id.trim().is_empty() && id.trim() == id)
        .ok_or(StorageError::Validation(
            "loom block_id must be non-empty without surrounding whitespace",
        ))?;
    let is_canvas = matches!(block.content_type, LoomBlockContentType::Canvas);
    // Master Spec LoomBlockContentType (11-shared-dev-platform §242-247) + the implemented canvas
    // kind; LM-RLS-001 gates creation by membership role, not by content type.
    if !matches!(
        block.content_type,
        LoomBlockContentType::Note
            | LoomBlockContentType::File
            | LoomBlockContentType::AnnotatedFile
            | LoomBlockContentType::TagHub
            | LoomBlockContentType::Journal
            | LoomBlockContentType::Canvas
    ) || is_canvas != board_state.is_some()
    {
        return Err(StorageError::Validation(
            "record-user Loom creation supports note, file, annotated_file, tag_hub, journal or canvas blocks",
        ));
    }
    if let Some(state) = &board_state {
        validate_board_state(state)?;
    }
    if metadata.resource_id != block_id {
        return Err(StorageError::Guard("guarded resource id mismatch"));
    }

    let timestamp = Datetime::from(metadata.timestamp);
    let preview = LoomBlock {
        block_id: block_id.clone(),
        workspace_id: workspace_id.to_owned(),
        content_type: block.content_type.clone(),
        document_id: block.document_id.clone(),
        asset_id: block.asset_id.clone(),
        title: block.title.clone(),
        original_filename: block.original_filename.clone(),
        content_hash: block.content_hash.clone(),
        pinned: block.pinned,
        favorite: false,
        pin_order: None,
        journal_date: block.journal_date.clone(),
        created_at: metadata.timestamp,
        updated_at: metadata.timestamp,
        imported_at: block.imported_at,
        derived: block.derived.clone(),
    };
    let entity_id = format!("KEN-{}", Uuid::now_v7().simple());
    let display_name = preview
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            preview
                .original_filename
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} {}", preview.content_type.as_str(), preview.block_id));
    let run_id = format!("LOOM-BRIDGE-{workspace_id}");
    let bridge_event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomBlockIndexed,
        bridge_actor_from_metadata(&metadata),
    )
    .aggregate("knowledge_loom_block", entity_id.clone())
    .idempotency_key(format!(
        "KEI-loom-bridge-{block_id}-{}",
        metadata.edit_event_id
    ))
    .source_component("loom_block_knowledge_bridge")
    .payload(json!({
        "type": "knowledge_loom_block_indexed",
        "workspace_id": workspace_id,
        "block_id": block_id,
        "entity_id": entity_id,
        "content_type": preview.content_type.as_str(),
        "extractor_version": EXTRACTOR_VERSION,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom bridge EventLedger receipt build failed"))?;
    let (_, bridge_event) = event_ledger::prepare_event(bridge_event)?;
    let board_event = board_state
        .as_ref()
        .map(|state| {
            let run_id = format!("LOOM-CANVAS-BOARD-{block_id}");
            NewKernelEvent::builder(
                run_id.clone(),
                run_id,
                KernelEventType::KnowledgeLoomCanvasBoardRecorded,
                bridge_actor_from_metadata(&metadata),
            )
            .aggregate("loom_canvas_board", block_id.clone())
            .idempotency_key(format!(
                "KEI-loom-canvas-{block_id}-{}",
                metadata.edit_event_id
            ))
            .source_component("loom_canvas_board")
            .payload(json!({
                "type": "knowledge_loom_canvas_board_recorded",
                "op": "create",
                "workspace_id": workspace_id,
                "block_id": block_id,
                "board_state": state,
            }))
            .build()
            .map_err(|_| StorageError::Validation("loom canvas EventLedger receipt build failed"))
            .and_then(|event| event_ledger::prepare_event(event).map(|(_, write)| write))
        })
        .transpose()?;
    let mut derived = serde_json::to_value(&preview.derived)?;
    if !derived.is_object() {
        derived = json!({});
    }
    let content = OwnedLoomBlockContent {
        block_id: block_id.clone(),
        workspace_id: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        created_in_session_id: RecordId::new("authenticated_sessions", scope.session_id.clone()),
        content_type: preview.content_type.as_str().to_owned(),
        document_id: preview
            .document_id
            .clone()
            .map(|id| RecordId::new("documents", id)),
        asset_id: preview
            .asset_id
            .clone()
            .map(|id| RecordId::new("assets", id)),
        title: preview.title.clone(),
        original_filename: preview.original_filename.clone(),
        content_hash: preview.content_hash.clone(),
        pinned: preview.pinned,
        favorite: false,
        pin_order: None,
        journal_date: preview.journal_date.clone(),
        last_job_id: metadata.job_id.map(|id| id.to_string()),
        last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
        last_actor_id: metadata.actor_id.clone(),
        edit_event_id: metadata.edit_event_id.to_string(),
        last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        created_at: timestamp,
        updated_at: timestamp,
        imported_at: preview.imported_at.map(Datetime::from),
        backlink_count: preview.derived.backlink_count,
        mention_count: preview.derived.mention_count,
        tag_count: preview.derived.tag_count,
        derived_json: derived,
        preview_status: preview.derived.preview_status.as_str().to_owned(),
        thumbnail_asset_id: preview
            .derived
            .thumbnail_asset_id
            .clone()
            .map(|id| RecordId::new("assets", id)),
        proxy_asset_id: preview
            .derived
            .proxy_asset_id
            .clone()
            .map(|id| RecordId::new("assets", id)),
    };
    let bindings = OwnedLoomBundleBindings {
        block: RecordId::new(BLOCKS, block_id.clone()),
        content,
        search: RecordId::new("loom_block_search_index", block_id.clone()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        content_type: preview.content_type.as_str().to_owned(),
        search_text: [
            preview.title.as_deref(),
            preview.original_filename.as_deref(),
            preview.derived.full_text_index.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n"),
        creator: RecordId::new("authenticated_sessions", scope.session_id),
        parent: RecordId::new("protected_resources", scope.resource_id),
        authorizing_grant: scope
            .grant_id
            .map(|id| RecordId::new("resource_grants", id)),
        owned_resource: RecordId::new("protected_resources", Uuid::now_v7().to_string()),
        owned_grant: RecordId::new("resource_grants", Uuid::now_v7().to_string()),
        locator_hash: hex::encode(Sha256::digest(format!("loom_block:{block_id}").as_bytes())),
        entity: RecordId::new(ENTITIES, entity_id.clone()),
        entity_id,
        display_name,
        detection_provenance: json!({
            "extractor": "loom_block_knowledge_bridge",
            "extractor_version": EXTRACTOR_VERSION,
            "method": "record_user_atomic_create",
            "content_type": preview.content_type.as_str(),
        }),
        bridge: RecordId::new(BRIDGES, block_id.clone()),
        bridge_event,
        board_event,
        board: is_canvas.then(|| RecordId::new(BOARDS, block_id.clone())),
        board_state,
        updated_at: timestamp,
    };
    #[cfg(test)]
    let execution = execute_record_user_loom_bundle_with_test_diagnostics(db, bindings);
    #[cfg(not(test))]
    let execution = db.execute_returning(OWNED_LOOM_BUNDLE_SQL, bindings);
    execution.await.map_err(|error| {
        let rendered = error.to_string();
        if rendered.contains("HSK-403-PROTECTED-RESOURCE") {
            StorageError::Guard("HSK-403-PROTECTED-RESOURCE")
        } else {
            StorageError::Database(rendered)
        }
    })?;
    loom_store::get_loom_block(db, workspace_id, &block_id).await
}

async fn create_canvas_board_attempt(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
    board_state: Value,
) -> StorageResult<LoomCanvasBoard> {
    let block = read_loom_block(storage, workspace_id, block_id).await?;
    if !matches!(block.content_type, LoomBlockContentType::Canvas) {
        return Err(StorageError::Validation(
            "canvas board block must be content_type=canvas",
        ));
    }
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
    // UPSERT=4, COMMIT=5, projection SELECT=6. A silently denied record-user receipt CREATE or
    // board UPSERT (MT-154 silent-deny ruling) THROWs the constant denial in place.
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
                         IF array::len((CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, \
                           actor_id: $event.actor_id, causation_id: $event.causation_id, \
                           correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                           source_component: $event.source_component, payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, \
                           created_at: $event.created_at \
                         } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
                         IF array::len((UPSERT $board SET block_id = $block, workspace_id = $workspace, \
                           board_state = $board_state, updated_at = time::now(), \
                           event_ledger_event_id = $event.record RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
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
    database: &SurrealDatabase,
    ctx: &WriteContext,
    workspace_id: &str,
    block_id: &str,
    board_state: Value,
    expected_event_ledger_event_id: &str,
) -> StorageResult<LoomCanvasBoard> {
    validate_board_state(&board_state)?;
    if expected_event_ledger_event_id.trim().is_empty()
        || expected_event_ledger_event_id.trim() != expected_event_ledger_event_id
    {
        return Err(StorageError::Validation(
            "canvas viewport requires an exact EventLedger revision",
        ));
    }
    validate_write(database.storage(), ctx, block_id).await?;
    let board_state = &board_state;
    database
        .guarded_mutation(
            vec![LockKey::record(BOARDS, block_id.to_owned())],
            Replay::idempotent(format!(
                "canvas-board-viewport:{block_id}:{expected_event_ledger_event_id}"
            )),
            None,
            || {
                update_canvas_board_state_attempt(
                    database.storage(),
                    workspace_id,
                    block_id,
                    board_state.clone(),
                    expected_event_ledger_event_id,
                )
            },
        )
        .await
}

async fn update_canvas_board_state_attempt(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
    board_state: Value,
    expected_event_ledger_event_id: &str,
) -> StorageResult<LoomCanvasBoard> {
    let event = prepare_canvas_event(block_id, workspace_id, "viewport", board_state.clone())?;
    let bindings = BoardWriteBindings {
        board: RecordId::new(BOARDS, block_id.to_owned()),
        block: RecordId::new(BLOCKS, block_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        board_state,
        expected_event: Some(RecordId::new(
            EVENT_LEDGER,
            expected_event_ledger_event_id.to_owned(),
        )),
        event,
    };
    // Result indexes: BEGIN=0, board guard=1, revision guard=2, event=3,
    // UPDATE=4, COMMIT=5, projection SELECT=6. A silently denied record-user receipt CREATE or
    // board UPDATE (MT-154 silent-deny ruling) THROWs the constant denial in place.
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
                         IF array::len((CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, \
                           actor_id: $event.actor_id, causation_id: $event.causation_id, \
                           correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                           source_component: $event.source_component, payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, \
                           created_at: $event.created_at \
                         } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
                         IF array::len((UPDATE $board SET board_state = $board_state, updated_at = time::now(), \
                           event_ledger_event_id = $event.record RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
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
    database: &SurrealDatabase,
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
    validate_write(database.storage(), ctx, &placement_id).await?;
    #[cfg(any(test, feature = "surreal-test-support"))]
    mark_stage_reference_writer_waiting(&placement.placed_block_id);
    // The placed block is the row a concurrent Stage compensation deletes and the
    // narrower half of `uq_loom_canvas_placement`; the placement id is fresh.
    let placement = &placement;
    let placement_id = &placement_id;
    database
        .guarded_mutation(
            vec![LockKey::record(BLOCKS, placement.placed_block_id.clone())],
            Replay::idempotent(format!("canvas-placement-create:{placement_id}")),
            None,
            || {
                place_block_on_canvas_attempt(
                    database.storage(),
                    placement_id.clone(),
                    placement.clone(),
                )
            },
        )
        .await
}

async fn place_block_on_canvas_attempt(
    storage: &SurrealStorage,
    placement_id: String,
    placement: NewLoomCanvasPlacement,
) -> StorageResult<LoomCanvasPlacement> {
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
                         IF array::len((CREATE $placement CONTENT { placement_id: $placement_id, \
                           canvas_block_id: $canvas, workspace_id: $workspace, placed_block_id: $placed_block, \
                           x: $x, y: $y, w: $w, h: $h, z_index: $z_index, group_id: $group_id, \
                           is_text_card: $is_text_card, stage_provenance_key: $stage_provenance_key, \
                           stage_provenance: NONE } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; };
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

pub(crate) async fn record_user_canvas_placement_identity(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    placement_id: &str,
) -> StorageResult<(String, String)> {
    let row: Option<PlacementRow> = db
        .query_first(
            "SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, x, y, w, h, z_index, group_id, is_text_card, created_at, updated_at FROM $record WHERE workspace_id = $workspace;",
            RecordWorkspaceBindings {
                record: RecordId::new(PLACEMENTS, placement_id.to_owned()),
                workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
            },
        )
        .await
        .map_err(map_err)?;
    let placement = row
        .map(placement_to_domain)
        .transpose()?
        .ok_or(StorageError::NotFound("loom_canvas_placement"))?;
    Ok((placement.canvas_block_id, placement.placed_block_id))
}

const RECORD_USER_CANVAS_PLACEMENT_SQL: &str = "BEGIN TRANSACTION; \
             LET $creator_account = $creator.account_id; LET $creator_principal = $creator.principal_id; LET $creator_space = $creator.access_space_id; \
             IF array::len((UPDATE $creator SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $board_resource SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $source_resource SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $board_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $source_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             LET $created_placement = (CREATE $placement CONTENT { placement_id: $placement_id, canvas_block_id: $canvas, workspace_id: $workspace, placed_block_id: $placed_block, x: $x, y: $y, w: $w, h: $h, z_index: $z_index, group_id: $group_id, is_text_card: $is_text_card, stage_provenance_key: NONE, stage_provenance: NONE } RETURN AFTER)[0]; \
             IF $created_placement = NONE OR $created_placement.workspace_id != $workspace OR $created_placement.canvas_block_id != $canvas OR $created_placement.placed_block_id != $placed_block { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             LET $created_event = (CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, created_at: $event.created_at } RETURN AFTER)[0]; \
             IF $created_event = NONE { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             COMMIT TRANSACTION; \
             RETURN { placement: $created_placement, event: { event_id: $created_event.event_id, event_sequence: $created_event.event_sequence, created_at: $created_event.created_at } };";

#[cfg(test)]
async fn execute_record_user_canvas_placement_with_test_diagnostics(
    db: &SurrealDataContext<'_>,
    bindings: RecordUserPlacementBindings,
) -> Result<Vec<PlacementCreateReceiptRow>, SurrealStorageError> {
    let mut response = db
        .client
        .query(RECORD_USER_CANVAS_PLACEMENT_SQL)
        .bind(SurrealValue::into_value(bindings))
        .await
        .map_err(|error| {
            eprintln!("record-user-canvas-placement phase=query error={error}");
            error
        })?;
    let mut errors = response.take_errors().into_iter().collect::<Vec<_>>();
    errors.sort_by_key(|(statement_index, _)| *statement_index);
    if !errors.is_empty() {
        let meaningful = errors
            .iter()
            .position(|(_, error)| {
                !error
                    .to_string()
                    .to_ascii_lowercase()
                    .contains("query was not executed due to a failed transaction")
            })
            .unwrap_or(0);
        let (statement_index, error) = errors.swap_remove(meaningful);
        eprintln!(
            "record-user-canvas-placement transactionfailed statement_index={statement_index} error={error}"
        );
        return Err(error.into());
    }
    response.take(10).map_err(|error| {
        eprintln!("record-user-canvas-placement phase=result_decode error={error}");
        error.into()
    })
}

pub(crate) async fn place_record_user_canvas_block(
    db: &SurrealDataContext<'_>,
    placement_id: String,
    placement: NewLoomCanvasPlacement,
    metadata: MutationMetadata,
    source_scope: super::resource_authority::RecordUserScope,
) -> StorageResult<LoomCanvasPlacementCreateReceipt> {
    validate_geometry(placement.w, placement.h)?;
    if placement.stage_provenance_key.is_some() {
        return Err(StorageError::Validation(
            "Stage provenance placements must use create_stage_canvas_card",
        ));
    }
    require_record_user_placement_scopes(
        &placement.workspace_id,
        &placement.canvas_block_id,
        &placement.placed_block_id,
        &source_scope,
    )?;
    if source_scope.workspace_id.as_deref() != Some(placement.placed_block_id.as_str()) {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    let board_scope =
        current_record_user_scope().ok_or(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"))?;
    let event = prepare_record_user_placement_event(
        &metadata,
        &placement_id,
        &placement.workspace_id,
        &placement.canvas_block_id,
        &placement.placed_block_id,
        "knowledge_loom_canvas_placement_recorded",
        "create",
    )?;
    let bindings = RecordUserPlacementBindings {
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
        creator: RecordId::new("authenticated_sessions", board_scope.session_id),
        board_resource: RecordId::new("protected_resources", board_scope.resource_id),
        source_resource: RecordId::new("protected_resources", source_scope.resource_id),
        board_grant: RecordId::new(
            "resource_grants",
            board_scope.grant_id.expect("scope checked"),
        ),
        source_grant: RecordId::new(
            "resource_grants",
            source_scope.grant_id.expect("scope checked"),
        ),
        event,
    };
    // The placement is created before its receipt so the receipt predicate
    // binds the immutable payload witnesses to this exact row. The transaction
    // rolls both writes back when either table permission rejects the request.
    #[cfg(test)]
    let execution = execute_record_user_canvas_placement_with_test_diagnostics(db, bindings);
    #[cfg(not(test))]
    let execution = db.query_values_at(RECORD_USER_CANVAS_PLACEMENT_SQL, bindings, 10);
    let rows: Vec<PlacementCreateReceiptRow> = execution.await.map_err(map_err)?;
    let row = rows.into_iter().next().ok_or_else(|| {
        StorageError::Database(
            "record-user Canvas placement transaction returned no receipt".to_owned(),
        )
    })?;
    Ok(LoomCanvasPlacementCreateReceipt {
        placement: placement_to_domain(row.placement)?,
        event: LoomMutationEventReceipt {
            event_id: row.event.event_id,
            event_sequence: row.event.event_sequence,
            created_at: row.event.created_at.into_inner(),
        },
    })
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

/// MT-153 AC-153-7 (Master Spec 02-system-architecture.md:2758/2773/2776; spec_ruling_c3_silent_deny):
/// the Stage card as ONE record-user transaction in the account's workspace `fs.write`/Create scope.
/// It keeps the legacy tuple (RichDocument, same-id Loom projection, search row, version 1, knowledge
/// entity + bridge + index receipt, Stage placement) and the same authority/board/key guards, and adds
/// what the non-Stage text-card path (`create_owned_document_rows`) mints: the RichDocument's
/// `created_in_session_id`, its `rich_document` protected resource and creator grant (RETURN NONE +
/// the explicit early check, because neither is selectable before the grant exists), then every
/// projection CREATE is checked for exactly one row so a permission-dropped write THROWs the constant
/// denial, and the authorization anchors are touched against a concurrent revocation.
/// Statements: BEGIN(0) auth(1) artifact(2) board(3) key(4) document(5) resource(6) grant(7)
/// early-check(8) block(9) search(10) version(11) entity(12) receipt(13) bridge(14) placement(15)
/// LET x3(16..18) touches(19) COMMIT(20) placement read-back(21).
const STAGE_RECORD_USER_CREATE_RESULT_INDEX: usize = 21;
const STAGE_RECORD_USER_CREATE_SQL: &str = "BEGIN TRANSACTION; \
    IF $creator = NONE OR $creator != $auth.id OR !fn::mt109_live_session() OR $parent = NONE OR $authorizing_grant = NONE { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF (SELECT VALUE id FROM $artifact WHERE workspace_id = $workspace AND content_sha256 = $provenance_sha256 AND manifest_ref = $provenance_manifest_ref AND correlation_id = $provenance_correlation_id)[0] = NONE { THROW 'HSK-CANVAS-STAGE-AUTHORITY'; }; \
    IF (SELECT VALUE id FROM $canvas WHERE workspace_id = $workspace)[0] = NONE { THROW 'HSK-CANVAS-BOARD-NOT-FOUND'; }; \
    IF array::len((SELECT id FROM loom_canvas_placements WHERE workspace_id = $workspace AND canvas_block_id = $canvas AND stage_provenance_key = $stage_provenance_key)) != 0 { THROW 'HSK-CANVAS-STAGE-PROVENANCE-CONFLICT'; }; \
    CREATE $document CONTENT { created_in_session_id: $creator, rich_document_id: $document_id, workspace_id: $workspace, document_id: NONE, title: $document_title, schema_version: $schema_version, doc_version: 1, content_json: $content_json, content_sha256: $content_sha256, crdt_document_id: NONE, crdt_snapshot_id: NONE, promotion_receipt_event_id: NONE, projection_refs: [], project_ref: NONE, folder_ref: NONE, authority_label: 'promoted', owner_actor_kind: NONE, owner_actor_id: NONE, deleted_at: NONE, created_at: $written_at, updated_at: $written_at } RETURN NONE; \
    CREATE $owned_resource SET resource_kind = 'rich_document', external_resource_id = $document_id, owner_account_id = $creator.account_id, created_by_principal_id = $creator.principal_id, created_in_session_id = $creator, creator_grant_id = $owned_grant, access_space_id = $creator.access_space_id, parent_resource_id = $parent, schema_version = 1, lifecycle_state = 'active', policy_version = $creator.policy_version, classification = 'account_private', storage_locator_hash = $locator_hash, created_at = time::now(), updated_at = time::now() RETURN NONE; \
    CREATE $owned_grant SET account_id = $creator.account_id, principal_id = $creator.principal_id, access_space_id = $creator.access_space_id, resource_id = $owned_resource, actions = ['read', 'create', 'update', 'delete'], capability_ids = ['fs.read', 'fs.write'], delegation_chain = $creator.delegation_chain, status = 'active', grant_version = 1, policy_version = $creator.policy_version, expires_at = NONE, revoked_at = NONE, created_at = time::now(), updated_at = time::now() RETURN NONE; \
    IF array::len((SELECT VALUE id FROM $document WHERE rich_document_id = $document_id AND workspace_id = $workspace AND content_sha256 = $content_sha256 AND doc_version = 1 AND created_in_session_id = $creator)) != 1 OR array::len((SELECT VALUE id FROM $owned_resource WHERE resource_kind = 'rich_document' AND owner_account_id = $creator.account_id AND created_by_principal_id = $creator.principal_id AND created_in_session_id = $creator AND creator_grant_id = $owned_grant AND access_space_id = $creator.access_space_id AND parent_resource_id = $parent AND lifecycle_state = 'active')) != 1 OR array::len((SELECT VALUE id FROM $owned_grant WHERE account_id = $creator.account_id AND principal_id = $creator.principal_id AND access_space_id = $creator.access_space_id AND resource_id = $owned_resource AND actions = ['read', 'create', 'update', 'delete'] AND capability_ids = ['fs.read', 'fs.write'] AND delegation_chain = $creator.delegation_chain AND status = 'active')) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $block CONTENT { block_id: $document_id, workspace_id: $workspace, source_rich_document_id: $document, content_type: 'note', document_id: NONE, asset_id: NONE, title: $document_title, original_filename: NONE, content_hash: $content_sha256, pinned: false, favorite: false, pin_order: NONE, journal_date: NONE, last_job_id: NONE, last_workflow_id: NONE, last_actor_id: $actor_id, edit_event_id: $edit_event_id, last_actor_kind: $actor_kind, created_at: $written_at, updated_at: $written_at, imported_at: NONE, backlink_count: 0, mention_count: 0, tag_count: 0, derived_json: $derived_json, preview_status: 'none', thumbnail_asset_id: NONE, proxy_asset_id: NONE } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $search CONTENT { block_id: $block, workspace_id: $workspace, content_type: 'note', search_text: $search_text, embedding: NONE, embedding_model: NONE, indexed_at: $written_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE knowledge_rich_document_versions CONTENT { rich_document_id: $document, doc_version: 1, schema_version: $schema_version, content_json: $content_json, content_sha256: $content_sha256, crdt_snapshot_id: NONE, promotion_receipt_event_id: NONE, created_at: $written_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $entity CONTENT { entity_id: $entity_id, workspace_id: $workspace, entity_kind: 'loom_block', entity_key: $document_id, display_name: $document_title, detection_provenance: $detection_provenance, lifecycle_state: 'active', primary_source_id: NONE, first_detected_in_run: NONE, last_detected_in_run: NONE, created_at: $written_at, updated_at: $written_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, created_at: $event.created_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $bridge CONTENT { block_id: $block, workspace_id: $workspace, entity_id: $entity, index_event_id: $event.record, created_at: $written_at, updated_at: $written_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((CREATE $placement CONTENT { placement_id: $placement_id, canvas_block_id: $canvas, workspace_id: $workspace, placed_block_id: $block, x: $x, y: $y, w: $w, h: $h, z_index: $z_index, group_id: NONE, is_text_card: true, stage_provenance_key: $stage_provenance_key, stage_provenance: $stage_provenance, created_at: $written_at, updated_at: $written_at } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    LET $creator_account = $creator.account_id; LET $creator_principal = $creator.principal_id; LET $creator_space = $creator.access_space_id; \
    IF array::len((UPDATE $creator SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $parent SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $authorizing_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    COMMIT TRANSACTION; \
    SELECT placement_id, canvas_block_id, workspace_id, placed_block_id, x, y, w, h, z_index, group_id, is_text_card, stage_provenance_key, stage_provenance, created_at, updated_at FROM $placement;";

pub(crate) async fn create_stage_canvas_card(
    database: &SurrealDatabase,
    ctx: &WriteContext,
    card: NewLoomCanvasStageCard,
) -> StorageResult<LoomCanvasStageCard> {
    if card.title.trim() != card.title || card.title.is_empty() {
        return Err(StorageError::Validation(
            "Stage canvas card title must be non-empty and trimmed",
        ));
    }
    validate_geometry(card.w, card.h)?;
    validated_stage_provenance(&card.stage_provenance_key, &card.stage_provenance)?;
    // The idempotent identity is (workspace, canvas, provenance key)
    // (`idx_loom_canvas_stage_provenance`); every row id the card creates is fresh.
    let card = &card;
    database
        .guarded_mutation(
            vec![LockKey::natural_key(
                card.workspace_id.clone(),
                "canvas_stage_provenance",
                format!("{}|{}", card.canvas_block_id, card.stage_provenance_key),
            )],
            Replay::idempotent(format!(
                "canvas-stage-card:{}:{}:{}",
                card.workspace_id, card.canvas_block_id, card.stage_provenance_key
            )),
            None,
            || create_stage_canvas_card_attempt(database.storage(), ctx, card),
        )
        .await
}

/// One attempt: the authority and replay reads plus the guarded transaction,
/// re-run together on an engine commit conflict.
async fn create_stage_canvas_card_attempt(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    card: &NewLoomCanvasStageCard,
) -> StorageResult<LoomCanvasStageCard> {
    use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
    use crate::knowledge_document::import::{import_snippet, ImportFormat};

    let stage_provenance =
        validated_stage_provenance(&card.stage_provenance_key, &card.stage_provenance)?;
    let canonical_markdown = serde_json::to_string(&card.stage_provenance)?;
    let imported = import_snippet(&canonical_markdown, ImportFormat::Markdown);

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
                card,
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
    // MT-153 AC-153-7: inside the account's workspace Create scope the card is written as the
    // record user, with the RichDocument's owned resource + creator grant minted in the same
    // transaction; without a scope (storage-level root proofs) the legacy statement runs unchanged.
    let witness = stage_record_user_witness(&card.workspace_id, &document_id)?;
    let record_user = witness.is_some();
    let bindings = StageCreateBindings {
        creator: witness.as_ref().map(|w| w.creator.clone()),
        parent: witness.as_ref().map(|w| w.parent.clone()),
        authorizing_grant: witness.as_ref().map(|w| w.authorizing_grant.clone()),
        owned_resource: witness.as_ref().map(|w| w.owned_resource.clone()),
        owned_grant: witness.as_ref().map(|w| w.owned_grant.clone()),
        locator_hash: witness.map(|w| w.locator_hash),
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
        // projection retains the legacy HUMAN/anonymous attribution. The projection
        // binds `source_rich_document_id` to the RichDocument it projects: MT-109's
        // `mt109_loom_source_integrity` event refuses a same-id block without that
        // link (HSK-MT109-LOOM-SOURCE-REQUIRED; MT-141 V2 red 420 / mt136 proof A).
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
                if record_user {
                    return database
                        .query_values_at(
                            STAGE_RECORD_USER_CREATE_SQL,
                            bindings,
                            STAGE_RECORD_USER_CREATE_RESULT_INDEX,
                        )
                        .await;
                }
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
                           source_rich_document_id: $document, \
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
                           payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, created_at: $event.created_at \
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

/// MT-153 AC-153-7: the Stage compensation as ONE record-user transaction in the account's workspace
/// `fs.write`/Delete scope. The six ownership/reference guards are the root statement's, verbatim; they
/// see the rows the owner can read. The complete reference guard is additionally evaluated with
/// permissions off by the `loom_canvas_placements` delete predicate
/// (`fn::mt154_stage_placement_delete` -> `fn::mt154_stage_card_unreferenced`, keyed by the compensation
/// receipt this transaction appends first), so a reference the owner cannot read still blocks the
/// compensation. A permission-dropped receipt or delete THROWs the constant denial (silent-deny ruling).
/// Statements: BEGIN(0) auth(1) guards(2..7) receipt(8) deletes(9..13) COMMIT(14) RETURN(15).
const STAGE_RECORD_USER_COMPENSATION_RESULT_INDEX: usize = 15;
const STAGE_RECORD_USER_COMPENSATION_SQL: &str = "BEGIN TRANSACTION; \
    IF $creator = NONE OR $creator != $auth.id OR !fn::mt109_live_session() { \
      THROW 'HSK-403-PROTECTED-RESOURCE'; \
    }; \
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
    IF array::len((CREATE $event.record CONTENT { event_id: $event.event_id, \
      event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, \
      session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
      aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
      event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, \
      causation_id: $event.causation_id, correlation_id: $event.correlation_id, \
      payload_hash: $event.payload_hash, source_component: $event.source_component, \
      payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, \
      authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, \
      authority_action: $event.authority_action, created_at: $event.created_at \
    } RETURN VALUE id)) != 1 { \
      THROW 'HSK-403-PROTECTED-RESOURCE'; \
    }; \
    IF array::len((DELETE $placement RETURN BEFORE)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((DELETE $bridge RETURN BEFORE)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((DELETE $entity RETURN BEFORE)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((DELETE $block RETURN BEFORE)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    IF array::len((DELETE $document RETURN BEFORE)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
    COMMIT TRANSACTION; \
    RETURN true;";

pub(crate) async fn compensate_stage_canvas_card(
    database: &SurrealDatabase,
    ctx: &WriteContext,
    card: CompensateLoomCanvasStageCard,
) -> StorageResult<LoomCanvasStageCompensation> {
    validated_stage_provenance(&card.stage_provenance_key, &card.stage_provenance)?;
    validate_write(database.storage(), ctx, &card.placement_id).await?;
    validate_write(database.storage(), ctx, &card.placed_block_id).await?;
    // Two records, acquired in sorted order: the placement being removed and the
    // placed block whose document/block/bridge/entity tuple is deleted with it.
    let card = &card;
    database
        .guarded_mutation(
            vec![
                LockKey::record(PLACEMENTS, card.placement_id.clone()),
                LockKey::record(BLOCKS, card.placed_block_id.clone()),
            ],
            Replay::idempotent(format!(
                "canvas-stage-compensate:{}:{}:{}",
                card.workspace_id, card.placement_id, card.stage_provenance_key
            )),
            None,
            || compensate_stage_canvas_card_attempt(database.storage(), ctx, card.clone()),
        )
        .await
}

/// One attempt: every ownership read and the guarded transaction, re-run
/// together on an engine commit conflict.
/// Compensation deletes the projection block BEFORE its RichDocument: the block's
/// `source_rich_document_id` link is `REFERENCE ON DELETE REJECT`, so the reverse order
/// is refused by the engine once the stage card carries the MT-109 source link.
async fn compensate_stage_canvas_card_attempt(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    card: CompensateLoomCanvasStageCard,
) -> StorageResult<LoomCanvasStageCompensation> {
    use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
    use crate::knowledge_document::import::{import_snippet, ImportFormat};

    let stage_provenance =
        validated_stage_provenance(&card.stage_provenance_key, &card.stage_provenance)?;
    // MT-153 AC-153-7: inside the account's workspace Delete scope every read below and the
    // compensation transaction run as the record user; without a scope the root proof path is unchanged.
    let creator = stage_compensation_creator(&card.workspace_id)?;
    let record_user = creator.is_some();

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

    #[cfg(any(test, feature = "surreal-test-support"))]
    pause_stage_compensation_after_validation(&card.placed_block_id).await;

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
        creator,
    };

    // Result indexes: BEGIN=0; six ownership/reference guards=1..6;
    // compensation event=7; five exact deletes=8..12; COMMIT=13.
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                if record_user {
                    return database
                        .query_values_at::<surrealdb::types::Value, _>(
                            STAGE_RECORD_USER_COMPENSATION_SQL,
                            bindings,
                            STAGE_RECORD_USER_COMPENSATION_RESULT_INDEX,
                        )
                        .await;
                }
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
                           payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, created_at: $event.created_at \
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
                         IF array::len((DELETE $block RETURN BEFORE)) != 1 { \
                           THROW 'HSK-CANVAS-STAGE-COMPENSATION-DELETE'; \
                         }; \
                         IF array::len((DELETE $document RETURN BEFORE)) != 1 { \
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
    database: &SurrealDatabase,
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
    validate_write(database.storage(), ctx, placement_id).await?;
    let update = &update;
    database
        .guarded_mutation(
            vec![LockKey::record(PLACEMENTS, placement_id.to_owned())],
            Replay::idempotent(format!("canvas-placement-update:{placement_id}")),
            None,
            || {
                update_canvas_placement_attempt(
                    database.storage(),
                    workspace_id,
                    placement_id,
                    update.clone(),
                )
            },
        )
        .await
}

async fn update_canvas_placement_attempt(
    storage: &SurrealStorage,
    workspace_id: &str,
    placement_id: &str,
    update: LoomCanvasPlacementUpdate,
) -> StorageResult<LoomCanvasPlacement> {
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
    // Result indexes: placement guard=0, silent-deny-guarded UPDATE=1, projection SELECT=2
    // (MT-154 silent-deny ruling: a dropped record-user UPDATE is the constant denial, not a
    // 200 carrying the unchanged row).
    let rows: Vec<PlacementRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "IF (SELECT VALUE id FROM $placement WHERE workspace_id = $workspace)[0] = NONE { \
                           THROW 'HSK-CANVAS-PLACEMENT-NOT-FOUND'; \
                         }; \
                         IF array::len((UPDATE $placement SET x = $x ?? x, y = $y ?? y, w = $w ?? w, h = $h ?? h, \
                           z_index = $z_index ?? z_index, \
                           group_id = IF $group_id_set { $group_id } ELSE { group_id }, \
                           updated_at = time::now() RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
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
    database: &SurrealDatabase,
    ctx: &WriteContext,
    workspace_id: &str,
    placement_id: &str,
) -> StorageResult<LoomCanvasPlacementRemovalReceipt> {
    validate_write(database.storage(), ctx, placement_id).await?;
    database
        .guarded_mutation(
            vec![LockKey::record(PLACEMENTS, placement_id.to_owned())],
            Replay::idempotent(format!("canvas-placement-remove:{placement_id}")),
            None,
            || remove_canvas_placement_attempt(database.storage(), ctx, workspace_id, placement_id),
        )
        .await
}

pub(crate) async fn remove_record_user_canvas_placement(
    db: &SurrealDataContext<'_>,
    workspace_id: String,
    placement_id: String,
    metadata: MutationMetadata,
    source_scope: super::resource_authority::RecordUserScope,
) -> StorageResult<LoomCanvasPlacementRemovalReceipt> {
    let (canvas_block_id, placed_block_id) =
        record_user_canvas_placement_identity(db, &workspace_id, &placement_id).await?;
    require_record_user_placement_scopes(
        &workspace_id,
        &canvas_block_id,
        &placed_block_id,
        &source_scope,
    )?;
    if source_scope.workspace_id.as_deref() != Some(placed_block_id.as_str()) {
        return Err(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"));
    }
    let board_scope =
        current_record_user_scope().ok_or(StorageError::Guard("HSK-403-PROTECTED-RESOURCE"))?;
    let event = prepare_record_user_placement_event(
        &metadata,
        &placement_id,
        &workspace_id,
        &canvas_block_id,
        &placed_block_id,
        "knowledge_loom_canvas_placement_removed",
        "remove_placement",
    )?;
    let bindings = RecordUserPlacementRemovalBindings {
        placement: RecordId::new(PLACEMENTS, placement_id.clone()),
        workspace: RecordId::new(WORKSPACES, workspace_id.clone()),
        canvas: RecordId::new(BOARDS, canvas_block_id.clone()),
        placed_block: RecordId::new(BLOCKS, placed_block_id.clone()),
        creator: RecordId::new("authenticated_sessions", board_scope.session_id),
        board_resource: RecordId::new("protected_resources", board_scope.resource_id),
        source_resource: RecordId::new("protected_resources", source_scope.resource_id),
        board_grant: RecordId::new(
            "resource_grants",
            board_scope.grant_id.expect("scope checked"),
        ),
        source_grant: RecordId::new(
            "resource_grants",
            source_scope.grant_id.expect("scope checked"),
        ),
        event,
    };
    // Receipt creation precedes deletion so its CREATE permission can bind the
    // event payload to the live placement. Later receipt reads use the stable
    // board/source witnesses in that payload after the placement is gone.
    let rows: Vec<MutationEventRow> = db
        .query_values_at(
            "BEGIN TRANSACTION; \
             LET $creator_account = $creator.account_id; LET $creator_principal = $creator.principal_id; LET $creator_space = $creator.access_space_id; \
             IF array::len((UPDATE $creator SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_account SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_principal SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $creator_space SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $board_resource SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $source_resource SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $board_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 OR array::len((UPDATE $source_grant SET authorization_touch_nonce = (authorization_touch_nonce ?? 0) + 1 RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             IF array::len((SELECT VALUE id FROM $placement WHERE workspace_id = $workspace AND canvas_block_id = $canvas AND placed_block_id = $placed_block)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             LET $created_event = (CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, created_at: $event.created_at } RETURN AFTER)[0]; \
             IF $created_event = NONE { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             LET $removed_placement = (DELETE $placement RETURN BEFORE)[0]; \
             IF $removed_placement = NONE OR $removed_placement.workspace_id != $workspace OR $removed_placement.canvas_block_id != $canvas OR $removed_placement.placed_block_id != $placed_block { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
             COMMIT TRANSACTION; \
             RETURN { event_id: $created_event.event_id, event_sequence: $created_event.event_sequence, created_at: $created_event.created_at };",
            bindings,
            11,
        )
        .await
        .map_err(map_err)?;
    let event = rows.into_iter().next().ok_or_else(|| {
        StorageError::Database("record-user Canvas placement removal receipt is missing".to_owned())
    })?;
    Ok(LoomCanvasPlacementRemovalReceipt {
        workspace_id,
        canvas_block_id,
        placement_id,
        placed_block_id,
        event: LoomMutationEventReceipt {
            event_id: event.event_id,
            event_sequence: event.event_sequence,
            created_at: event.created_at.into_inner(),
        },
    })
}

async fn remove_canvas_placement_attempt(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    placement_id: &str,
) -> StorageResult<LoomCanvasPlacementRemovalReceipt> {
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
                         IF array::len((CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, \
                           session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, \
                           aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, \
                           event_type: $event.event_type, actor_kind: $event.actor_kind, \
                           actor_id: $event.actor_id, causation_id: $event.causation_id, \
                           correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, \
                           source_component: $event.source_component, payload: $event.payload, wsids: $event.wsids, authority_resource_id: $event.authority_resource_id, authority_session_id: $event.authority_session_id, authority_capability_id: $event.authority_capability_id, authority_action: $event.authority_action, \
                           created_at: $event.created_at \
                         } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
                         IF array::len((DELETE $placement RETURN BEFORE)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
                         COMMIT TRANSACTION; \
                         SELECT event_id, event_sequence, created_at FROM $event.record;",
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
    Ok(LoomCanvasPlacementRemovalReceipt {
        workspace_id: placement.workspace_id,
        canvas_block_id: placement.canvas_block_id,
        placement_id: placement.placement_id,
        placed_block_id: placement.placed_block_id,
        event: LoomMutationEventReceipt {
            event_id: event.event_id,
            event_sequence: event.event_sequence,
            created_at: event.created_at.into_inner(),
        },
    })
}

pub(crate) async fn add_canvas_visual_edge(
    database: &SurrealDatabase,
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
    validate_write(database.storage(), ctx, &visual_edge_id).await?;
    // Both endpoint placements (a removal of either races the edge's existence
    // guard); `acquire_many` orders them, so opposite-order edges cannot deadlock.
    let visual_edge_id = &visual_edge_id;
    let label = &label;
    database
        .guarded_mutation(
            vec![
                LockKey::record(PLACEMENTS, from_placement_id.to_owned()),
                LockKey::record(PLACEMENTS, to_placement_id.to_owned()),
            ],
            Replay::idempotent(format!("canvas-visual-edge-add:{visual_edge_id}")),
            None,
            || {
                add_canvas_visual_edge_attempt(
                    database.storage(),
                    workspace_id,
                    canvas_block_id,
                    from_placement_id,
                    to_placement_id,
                    label.clone(),
                    visual_edge_id.clone(),
                )
            },
        )
        .await
}

async fn add_canvas_visual_edge_attempt(
    storage: &SurrealStorage,
    workspace_id: &str,
    canvas_block_id: &str,
    from_placement_id: &str,
    to_placement_id: &str,
    label: Option<String>,
    visual_edge_id: String,
) -> StorageResult<LoomCanvasVisualEdge> {
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
                         IF array::len((CREATE $edge CONTENT { visual_edge_id: $visual_edge_id, \
                           canvas_block_id: $canvas, workspace_id: $workspace, \
                           from_placement_id: $from_placement, to_placement_id: $to_placement, \
                           label: $label } RETURN VALUE id)) != 1 { THROW 'HSK-403-PROTECTED-RESOURCE'; }; \
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

#[derive(SurrealValue)]
struct JournalLookupBindings {
    workspace: RecordId,
    journal_date: String,
}

#[derive(SurrealValue)]
struct JournalIdRow {
    block_id: String,
}

/// The existing daily-journal block id for `(workspace, date)`, used only to locate the note; the
/// API re-authorizes the read through the account's exact grant before returning any content.
pub(crate) async fn journal_block_id(
    storage: &SurrealStorage,
    workspace_id: &str,
    journal_date: &str,
) -> StorageResult<Option<String>> {
    let bindings = JournalLookupBindings {
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
        journal_date: journal_date.to_owned(),
    };
    let rows: Vec<JournalIdRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT block_id FROM loom_blocks WHERE workspace_id = $workspace \
                           AND content_type = 'journal' AND journal_date = $journal_date LIMIT 1;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    Ok(rows.into_iter().next().map(|row| row.block_id))
}

/// The canvas board that owns `visual_edge_id` inside `workspace_id`, read for API-boundary
/// authorization of a visual-edge removal. `None` for an unknown edge or a workspace mismatch.
pub(crate) async fn canvas_visual_edge_board_id(
    storage: &SurrealStorage,
    workspace_id: &str,
    visual_edge_id: &str,
) -> StorageResult<Option<String>> {
    let bindings = RecordWorkspaceBindings {
        record: RecordId::new(VISUAL_EDGES, visual_edge_id.to_owned()),
        workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
    };
    let rows: Vec<VisualEdgeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values(
                        "SELECT visual_edge_id, canvas_block_id, workspace_id, from_placement_id, \n                           to_placement_id, label, created_at FROM $record \n                         WHERE workspace_id = $workspace;",
                        bindings,
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    match rows.into_iter().next() {
        Some(row) => Ok(Some(visual_edge_to_domain(row)?.canvas_block_id)),
        None => Ok(None),
    }
}

pub(crate) async fn remove_canvas_visual_edge(
    database: &SurrealDatabase,
    ctx: &WriteContext,
    workspace_id: &str,
    visual_edge_id: &str,
) -> StorageResult<()> {
    validate_write(database.storage(), ctx, visual_edge_id).await?;
    let count = database
        .guarded_mutation(
            vec![LockKey::record(VISUAL_EDGES, visual_edge_id.to_owned())],
            Replay::idempotent(format!("canvas-visual-edge-remove:{visual_edge_id}")),
            None,
            || {
                let bindings = RecordWorkspaceBindings {
                    record: RecordId::new(VISUAL_EDGES, visual_edge_id.to_owned()),
                    workspace: RecordId::new(WORKSPACES, workspace_id.to_owned()),
                };
                async move {
                    database
                        .storage()
                        .with_data_operation(move |database| {
                            Box::pin(async move {
                                // MT-154 silent-deny ruling: a dropped record-user DELETE of a
                                // still-readable edge is the constant denial, not a 404.
                                database
                                    .query_values_at::<surrealdb::types::Value, _>(
                                        "LET $deleted = (DELETE $record WHERE workspace_id = $workspace RETURN BEFORE); IF array::len($deleted) = 0 AND (SELECT VALUE id FROM $record WHERE workspace_id = $workspace)[0] != NONE { THROW 'HSK-403-PROTECTED-RESOURCE'; } ELSE { RETURN $deleted; };",
                                        bindings,
                                        1,
                                    )
                                    .await
                                    .map(|rows| rows.len())
                            })
                        })
                        .await
                        .map_err(map_err)
                }
            },
        )
        .await?;
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
    struct EventRecordBinding {
        record: RecordId,
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
        let db = SurrealDatabase::new(store.clone());
        create_canvas_board(&db, &context(), workspace_id, canvas_id, board_state(0.0))
            .await
            .expect("create Canvas board")
    }

    #[tokio::test]
    async fn viewport_compare_and_swap_rejects_stale_event_revision() {
        let (_temp, store) = open_store().await;
        let workspace_id = "canvas-cas-workspace";
        let canvas_id = "canvas-cas";
        let created = create_board_fixture(&store, workspace_id, canvas_id).await;
        let db = SurrealDatabase::new(store.clone());

        let updated = update_canvas_board_state(
            &db,
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
            &db,
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
        let db = SurrealDatabase::new(store.clone());
        let placement = place_block_on_canvas(
            &db,
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
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await
        .expect("place source block");

        let receipt =
            remove_canvas_placement(&db, &context(), workspace_id, &placement.placement_id)
                .await
                .expect("remove placement");
        assert_eq!(receipt.workspace_id, workspace_id);
        assert_eq!(receipt.canvas_block_id, canvas_id);
        assert_eq!(receipt.placement_id, placement.placement_id);
        assert_eq!(receipt.placed_block_id, source_id);

        let event = store
            .with_data_operation({
                let event_id = receipt.event.event_id.clone();
                move |db| {
                    Box::pin(async move {
                        db.query_first::<RemovalEventProofRow, _>(
                            "SELECT event_id, event_sequence, payload FROM $record;",
                            EventRecordBinding {
                                record: RecordId::new(EVENT_LEDGER, event_id),
                            },
                        )
                        .await
                    })
                }
            })
            .await
            .expect("read removal event")
            .expect("removal event exists");
        assert_eq!(event.event_id, receipt.event.event_id);
        assert_eq!(event.event_sequence, receipt.event.event_sequence);
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
