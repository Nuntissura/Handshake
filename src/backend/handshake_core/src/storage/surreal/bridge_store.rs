//! LoomBlock to ProjectKnowledgeIndex authority bridge for embedded SurrealDB.

use serde_json::json;
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{event_ledger, loom_store, SurrealStorage};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::{
    LoomKnowledgeBridge, MutationMetadata, StorageError, StorageResult, WriteActorKind,
    WriteContext,
};

const BLOCKS: &str = "loom_blocks";
const ENTITIES: &str = "knowledge_entities";
const BRIDGES: &str = "loom_block_knowledge_bridge";
const EXTRACTOR_VERSION: &str = "loom_block_knowledge_bridge_v1";

// The embedded engine is single-process. Serialize the read/choose-id/write
// bridge path so two first-time bridge calls cannot choose different entity
// ids before the natural-identity index becomes visible.
static BRIDGE_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

#[derive(SurrealValue)]
struct EntityLookupBindings {
    workspace: RecordId,
    block_id: String,
}

#[derive(SurrealValue)]
struct EntityLookupRow {
    entity_id: String,
}

#[derive(SurrealValue)]
struct BridgeWriteBindings {
    entity_record: RecordId,
    bridge_record: RecordId,
    workspace: RecordId,
    block: RecordId,
    entity_id: String,
    block_id: String,
    display_name: String,
    detection_provenance: serde_json::Value,
    updated_at: Datetime,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct BridgeLookupBindings {
    workspace: RecordId,
    block: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct BridgeRow {
    block_id: RecordId,
    workspace_id: RecordId,
    entity_id: RecordId,
    index_event_id: RecordId,
    created_at: Datetime,
    updated_at: Datetime,
}

pub(crate) async fn bridge_loom_block_to_knowledge(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    metadata: MutationMetadata,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<LoomKnowledgeBridge> {
    if metadata.resource_id != block_id {
        return Err(StorageError::Guard("guarded resource id mismatch"));
    }

    let _mutation_guard = BRIDGE_MUTATION_LOCK.lock().await;
    let workspace_id_owned = workspace_id.to_owned();
    let block_id_owned = block_id.to_owned();
    let block = storage
        .with_storage_operation({
            let workspace_id = workspace_id_owned.clone();
            let block_id = block_id_owned.clone();
            move |database| {
                Box::pin(async move {
                    loom_store::get_loom_block(&database, &workspace_id, &block_id).await
                })
            }
        })
        .await
        .map_err(StorageError::from)??;

    let existing: Option<EntityLookupRow> = storage
        .with_data_operation({
            let workspace = RecordId::new("workspaces", workspace_id_owned.clone());
            let block_id = block_id_owned.clone();
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT entity_id FROM knowledge_entities \
                             WHERE workspace_id = $workspace AND entity_kind = 'loom_block' \
                             AND entity_key = $block_id LIMIT 1;",
                            EntityLookupBindings {
                                workspace,
                                block_id,
                            },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(StorageError::from)?;

    let entity_id = existing
        .map(|row| row.entity_id)
        .unwrap_or_else(|| format!("KEN-{}", Uuid::now_v7().simple()));
    let display_name = block
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            block
                .original_filename
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id));
    let actor = bridge_actor(ctx);
    let run_id = format!("LOOM-BRIDGE-{workspace_id}");
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
        metadata.timestamp.timestamp_nanos_opt().unwrap_or_default()
    ))
    .source_component("loom_block_knowledge_bridge")
    .payload(json!({
        "type": "knowledge_loom_block_indexed",
        "workspace_id": workspace_id,
        "block_id": block.block_id,
        "entity_id": entity_id,
        "content_type": block.content_type.as_str(),
        "extractor_version": EXTRACTOR_VERSION,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom bridge EventLedger receipt build failed"))?;
    let (_, event) = event_ledger::prepare_event(event)?;
    let bindings = BridgeWriteBindings {
        entity_record: RecordId::new(ENTITIES, entity_id.clone()),
        bridge_record: RecordId::new(BRIDGES, block_id_owned.clone()),
        workspace: RecordId::new("workspaces", workspace_id_owned),
        block: RecordId::new(BLOCKS, block_id_owned.clone()),
        entity_id,
        block_id: block_id_owned,
        display_name,
        detection_provenance: json!({
            "extractor": "loom_block_knowledge_bridge",
            "extractor_version": EXTRACTOR_VERSION,
            "method": "mt177_bridge",
            "content_type": block.content_type.as_str(),
        }),
        updated_at: Datetime::from(metadata.timestamp),
        event,
    };

    let rows: Vec<BridgeRow> = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at(
                        "BEGIN TRANSACTION; \
                         LET $identity = (SELECT VALUE entity_id FROM knowledge_entities \
                           WHERE workspace_id = $workspace AND entity_kind = 'loom_block' \
                           AND entity_key = $block_id LIMIT 1)[0]; \
                         IF $identity != NONE AND $identity != $entity_id { \
                           THROW 'HSK-LOOM-BRIDGE-IDENTITY-CONFLICT'; \
                         }; \
                         UPSERT $entity_record SET entity_id = $entity_id, workspace_id = $workspace, \
                           entity_kind = 'loom_block', entity_key = $block_id, display_name = $display_name, \
                           detection_provenance = $detection_provenance, lifecycle_state = 'active', \
                           updated_at = $updated_at; \
                         CREATE $event.record CONTENT { \
                           event_id: $event.event_id, event_version: $event.event_version, \
                           kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, \
                           aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, \
                           idempotency_key: $event.idempotency_key, event_type: $event.event_type, \
                           actor_kind: $event.actor_kind, actor_id: $event.actor_id, \
                           causation_id: $event.causation_id, correlation_id: $event.correlation_id, \
                           payload_hash: $event.payload_hash, source_component: $event.source_component, \
                           payload: $event.payload, created_at: $event.created_at \
                         }; \
                         UPSERT $bridge_record SET block_id = $block, workspace_id = $workspace, \
                           entity_id = $entity_record, index_event_id = $event.record, updated_at = $updated_at; \
                         COMMIT TRANSACTION; \
                         SELECT block_id, workspace_id, entity_id, index_event_id, created_at, updated_at \
                           FROM $bridge_record;",
                        bindings,
                        7,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| StorageError::Database(error.to_string()))?;
    rows.into_iter()
        .next()
        .map(map_bridge)
        .transpose()?
        .ok_or_else(|| StorageError::Database("loom bridge write returned no row".to_owned()))
}

pub(crate) async fn get_loom_block_knowledge_bridge(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Option<LoomKnowledgeBridge>> {
    let row: Option<BridgeRow> = storage
        .with_data_operation({
            let bindings = BridgeLookupBindings {
                workspace: RecordId::new("workspaces", workspace_id.to_owned()),
                block: RecordId::new(BLOCKS, block_id.to_owned()),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first(
                            "SELECT block_id, workspace_id, entity_id, index_event_id, created_at, updated_at \
                             FROM loom_block_knowledge_bridge \
                             WHERE workspace_id = $workspace AND block_id = $block LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(|error| StorageError::Database(error.to_string()))?;
    row.map(map_bridge).transpose()
}

pub(crate) async fn list_loom_block_knowledge_bridges(
    storage: &SurrealStorage,
    workspace_id: &str,
) -> StorageResult<Vec<LoomKnowledgeBridge>> {
    let rows: Vec<BridgeRow> = storage
        .with_data_operation({
            let workspace = RecordId::new("workspaces", workspace_id.to_owned());
            move |database| {
                Box::pin(async move {
                    database
                        .query_values(
                            "SELECT block_id, workspace_id, entity_id, index_event_id, created_at, updated_at \
                             FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace \
                             ORDER BY created_at ASC, block_id ASC;",
                            WorkspaceBinding { workspace },
                        )
                        .await
                })
            }
        })
        .await
        .map_err(|error| StorageError::Database(error.to_string()))?;
    rows.into_iter().map(map_bridge).collect()
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

fn map_bridge(row: BridgeRow) -> StorageResult<LoomKnowledgeBridge> {
    Ok(LoomKnowledgeBridge {
        block_id: record_key(row.block_id, BLOCKS)?,
        workspace_id: record_key(row.workspace_id, "workspaces")?,
        entity_id: record_key(row.entity_id, ENTITIES)?,
        index_event_id: record_key(row.index_event_id, "kernel_event_ledger")?,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
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
