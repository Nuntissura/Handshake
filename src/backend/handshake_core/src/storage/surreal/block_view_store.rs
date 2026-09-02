//! Embedded SurrealDB authority for saved Loom block-collection views.
//!
//! The create path deliberately keeps the legacy atomic boundary: the typed
//! `view_def` block, search projection, ProjectKnowledgeIndex bridge, both
//! EventLedger receipts, and the recoverable Flight Recorder outbox commit in
//! one transaction. The dedicated `view_definition_json` field is retained;
//! view definitions are never overloaded into `derived_json`.

use std::collections::{HashMap, HashSet};

use serde_json::{json, Value as JsonValue};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

#[cfg(any(test, feature = "surreal-test-support"))]
use super::SurrealStorage;
use super::{event_ledger, loom_store, SurrealDataContext, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::block_view_outbox::{self, BlockViewMutationOperation};
use crate::storage::{
    BlockViewDefinition, BlockViewField, BlockViewGroupBy, BlockViewKind, BlockViewLane,
    BlockViewRecord, BlockViewResults, BlockViewSortDirection, LoomBlock, LoomBlockContentType,
    LoomBlockDerived, MutationMetadata, PreviewStatus, StorageError, StorageResult, WriteActorKind,
    BLOCK_VIEW_UNTAGGED_LANE,
};

const BLOCKS: &str = "loom_blocks";
const WORKSPACES: &str = "workspaces";
const SEARCH: &str = "loom_block_search_index";
const ENTITIES: &str = "knowledge_entities";
const BRIDGES: &str = "loom_block_knowledge_bridge";
const OUTBOX: &str = "loom_block_view_fr_outbox";
const BRIDGE_EXTRACTOR_VERSION: &str = "loom_block_knowledge_bridge_v1";

// The embedded engine is single-process. This lock closes the read/create race
// for idempotent create and for the knowledge-entity natural identity.
static BLOCK_VIEW_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());

fn thing(table: &str, id: impl Into<String>) -> RecordId {
    RecordId::new(table, id.into())
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

fn map_err(error: SurrealStorageError) -> StorageError {
    let message = error.to_string();
    if message.contains("HSK-BLOCK-VIEW-NOT-FOUND") {
        StorageError::NotFound("loom_block")
    } else if message.contains("HSK-BLOCK-VIEW-CONFLICT") {
        StorageError::Conflict("loom_block_view_id")
    } else {
        StorageError::Database(message)
    }
}

fn require_resource(metadata: &MutationMetadata, block_id: &str) -> StorageResult<()> {
    if metadata.resource_id != block_id {
        return Err(StorageError::Guard("guarded resource id mismatch"));
    }
    Ok(())
}

fn encode_definition(definition: &BlockViewDefinition) -> StorageResult<String> {
    let encoded = serde_json::to_string(definition)?;
    // Decode the exact persisted representation before it enters authority.
    // This retains the legacy validation boundary without inventing stricter
    // cross-field rules that the PostgreSQL implementation never imposed.
    let _: BlockViewDefinition = serde_json::from_str(&encoded)?;
    Ok(encoded)
}

fn decode_definition(encoded: &str) -> StorageResult<BlockViewDefinition> {
    serde_json::from_str(encoded).map_err(StorageError::from)
}

fn event_hash(event: &crate::flight_recorder::FlightRecorderEvent) -> StorageResult<String> {
    let value = serde_json::to_value(event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    Ok(crate::kernel::context_bundle::sha256_hex(
        &crate::kernel::context_bundle::canonical_json_bytes(&value),
    ))
}

fn mutation_event(
    workspace_id: &str,
    block_id: &str,
    operation: &'static str,
) -> StorageResult<NewKernelEvent> {
    let run_id = format!("LOOM-BLOCK-{workspace_id}");
    NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomBlockMutated,
        KernelActor::System("loom-block".to_owned()),
    )
    .aggregate("loom_block", block_id.to_owned())
    .source_component("loom_block")
    .payload(json!({
        "type": "knowledge_loom_block_mutated",
        "schema_id": "hsk.loom_block_mutation@1",
        "workspace_id": workspace_id,
        "block_id": block_id,
        "operation": operation,
        "content_type": "view_def",
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom block-view mutation event build failed"))
}

fn bridge_actor(metadata: &MutationMetadata) -> KernelActor {
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

fn bridge_event(
    metadata: &MutationMetadata,
    workspace_id: &str,
    block_id: &str,
    entity_id: &str,
) -> StorageResult<NewKernelEvent> {
    let run_id = format!("LOOM-BRIDGE-{workspace_id}");
    NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomBlockIndexed,
        bridge_actor(metadata),
    )
    .aggregate("knowledge_loom_block", entity_id.to_owned())
    .idempotency_key(format!(
        "KEI-loom-bridge-{}-{}",
        entity_id,
        metadata.timestamp.timestamp_nanos_opt().unwrap_or_default()
    ))
    .source_component("loom_block_knowledge_bridge")
    .payload(json!({
        "type": "knowledge_loom_block_indexed",
        "workspace_id": workspace_id,
        "block_id": block_id,
        "entity_id": entity_id,
        "content_type": "view_def",
        "extractor_version": BRIDGE_EXTRACTOR_VERSION,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom block-view bridge event build failed"))
}

#[derive(SurrealValue)]
struct ExistingViewRow {
    workspace_id: RecordId,
    content_type: String,
    title: Option<String>,
    view_definition_json: Option<String>,
}

#[derive(SurrealValue)]
struct RecordBinding {
    record: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceRecordBinding {
    workspace: RecordId,
    record: RecordId,
}

#[derive(SurrealValue)]
struct EventIdRow {
    event_id: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct PublicationLookupBinding {
    workspace: RecordId,
    block: RecordId,
}

async fn existing_view(
    db: &SurrealDataContext<'_>,
    block_id: &str,
) -> StorageResult<Option<ExistingViewRow>> {
    db.query_first(
        "SELECT workspace_id, content_type, title, view_definition_json FROM $record LIMIT 1;",
        RecordBinding {
            record: thing(BLOCKS, block_id),
        },
    )
    .await
    .map_err(map_err)
}

async fn prior_create_publication(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<Option<Uuid>> {
    let row: Option<EventIdRow> = db
        .query_first(
            "SELECT event_id, created_at FROM loom_block_view_fr_outbox WHERE workspace_id = $workspace \
             AND block_id = $block AND operation = 'create' \
             ORDER BY created_at DESC, event_id DESC LIMIT 1;",
            PublicationLookupBinding {
                workspace: thing(WORKSPACES, workspace_id),
                block: thing(BLOCKS, block_id),
            },
        )
        .await
        .map_err(map_err)?;
    row.map(|row| {
        Uuid::parse_str(&row.event_id)
            .map_err(|error| StorageError::Serialization(error.to_string()))
    })
    .transpose()
}

#[derive(SurrealValue)]
struct BlockCreateContent {
    block_id: String,
    workspace_id: RecordId,
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
    derived_json: JsonValue,
    preview_status: String,
    thumbnail_asset_id: Option<RecordId>,
    proxy_asset_id: Option<RecordId>,
    view_definition_json: String,
}

#[derive(SurrealValue)]
struct OutboxContent {
    event_id: String,
    workspace_id: RecordId,
    block_id: RecordId,
    operation: String,
    event: JsonValue,
    event_hash: String,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct CreateBindings {
    block: RecordId,
    content: BlockCreateContent,
    search: RecordId,
    workspace: RecordId,
    search_text: String,
    entity: RecordId,
    entity_id: String,
    display_name: String,
    detection_provenance: JsonValue,
    bridge: RecordId,
    bridge_event: event_ledger::LedgerWrite,
    mutation_event: event_ledger::LedgerWrite,
    outbox: RecordId,
    outbox_content: OutboxContent,
}

#[derive(SurrealValue)]
struct MutationRow {
    block_id: String,
}

const CREATE_TRANSACTION: &str = "BEGIN TRANSACTION; \
    CREATE $block CONTENT $content RETURN AFTER; \
    UPSERT $search SET block_id = $block, workspace_id = $workspace, content_type = 'view_def', search_text = $search_text, indexed_at = time::now(); \
    CREATE $bridge_event.record CONTENT { event_id: $bridge_event.event_id, event_version: $bridge_event.event_version, kernel_task_run_id: $bridge_event.kernel_task_run_id, session_run_id: $bridge_event.session_run_id, aggregate_type: $bridge_event.aggregate_type, aggregate_id: $bridge_event.aggregate_id, idempotency_key: $bridge_event.idempotency_key, event_type: $bridge_event.event_type, actor_kind: $bridge_event.actor_kind, actor_id: $bridge_event.actor_id, causation_id: $bridge_event.causation_id, correlation_id: $bridge_event.correlation_id, payload_hash: $bridge_event.payload_hash, source_component: $bridge_event.source_component, payload: $bridge_event.payload, created_at: $bridge_event.created_at }; \
    CREATE $mutation_event.record CONTENT { event_id: $mutation_event.event_id, event_version: $mutation_event.event_version, kernel_task_run_id: $mutation_event.kernel_task_run_id, session_run_id: $mutation_event.session_run_id, aggregate_type: $mutation_event.aggregate_type, aggregate_id: $mutation_event.aggregate_id, idempotency_key: $mutation_event.idempotency_key, event_type: $mutation_event.event_type, actor_kind: $mutation_event.actor_kind, actor_id: $mutation_event.actor_id, causation_id: $mutation_event.causation_id, correlation_id: $mutation_event.correlation_id, payload_hash: $mutation_event.payload_hash, source_component: $mutation_event.source_component, payload: $mutation_event.payload, created_at: $mutation_event.created_at }; \
    CREATE $entity CONTENT { entity_id: $entity_id, workspace_id: $workspace, entity_kind: 'loom_block', entity_key: record::id($block), display_name: $display_name, detection_provenance: $detection_provenance, lifecycle_state: 'active', updated_at: $content.updated_at }; \
    CREATE $bridge CONTENT { block_id: $block, workspace_id: $workspace, entity_id: $entity, index_event_id: $bridge_event.record, updated_at: $content.updated_at }; \
    CREATE $outbox CONTENT $outbox_content; \
    UPDATE $block SET event_ledger_event_id = $mutation_event.record RETURN AFTER; \
    COMMIT TRANSACTION;";

#[cfg(any(test, feature = "surreal-test-support"))]
impl SurrealStorage {
    async fn set_block_view_create_failpoint(&self, statement: &'static str) -> StorageResult<()> {
        self.with_data_operation(move |database| {
            Box::pin(async move {
                database.client.query(statement).await?.check()?;
                Ok(())
            })
        })
        .await
        .map_err(map_err)
    }

    pub async fn test_set_block_view_block_create_failpoint(
        &self,
        enabled: bool,
    ) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_block_create_failpoint ON TABLE loom_blocks \
             WHEN $event = 'CREATE' THEN { THROW 'MT141-BLOCK-VIEW-BLOCK-CREATE'; };"
        } else {
            "REMOVE EVENT mt141_block_view_block_create_failpoint ON TABLE loom_blocks;"
        })
        .await
    }

    pub async fn test_set_block_view_search_failpoint(&self, enabled: bool) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_search_failpoint \
             ON TABLE loom_block_search_index WHEN ($event = 'CREATE' OR $event = 'UPDATE') \
             THEN { THROW 'MT141-BLOCK-VIEW-SEARCH'; };"
        } else {
            "REMOVE EVENT mt141_block_view_search_failpoint ON TABLE loom_block_search_index;"
        })
        .await
    }

    pub async fn test_set_block_view_bridge_receipt_failpoint(
        &self,
        enabled: bool,
    ) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_bridge_receipt_failpoint \
             ON TABLE kernel_event_ledger WHEN $event = 'CREATE' \
             AND $after.event_type = 'KNOWLEDGE_LOOM_BLOCK_INDEXED' \
             THEN { THROW 'MT141-BLOCK-VIEW-BRIDGE-RECEIPT'; };"
        } else {
            "REMOVE EVENT mt141_block_view_bridge_receipt_failpoint ON TABLE kernel_event_ledger;"
        })
        .await
    }

    pub async fn test_set_block_view_mutation_receipt_failpoint(
        &self,
        enabled: bool,
    ) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_mutation_receipt_failpoint \
             ON TABLE kernel_event_ledger WHEN $event = 'CREATE' \
             AND $after.event_type = 'KNOWLEDGE_LOOM_BLOCK_MUTATED' \
             THEN { THROW 'MT141-BLOCK-VIEW-MUTATION-RECEIPT'; };"
        } else {
            "REMOVE EVENT mt141_block_view_mutation_receipt_failpoint ON TABLE kernel_event_ledger;"
        })
        .await
    }

    pub async fn test_set_block_view_entity_failpoint(&self, enabled: bool) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_entity_failpoint ON TABLE knowledge_entities \
             WHEN $event = 'CREATE' THEN { THROW 'MT141-BLOCK-VIEW-ENTITY'; };"
        } else {
            "REMOVE EVENT mt141_block_view_entity_failpoint ON TABLE knowledge_entities;"
        })
        .await
    }

    pub async fn test_set_block_view_bridge_failpoint(&self, enabled: bool) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_bridge_failpoint \
             ON TABLE loom_block_knowledge_bridge WHEN $event = 'CREATE' \
             THEN { THROW 'MT141-BLOCK-VIEW-BRIDGE'; };"
        } else {
            "REMOVE EVENT mt141_block_view_bridge_failpoint ON TABLE loom_block_knowledge_bridge;"
        })
        .await
    }

    pub async fn test_set_block_view_outbox_failpoint(&self, enabled: bool) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_outbox_failpoint \
             ON TABLE loom_block_view_fr_outbox WHEN $event = 'CREATE' \
             THEN { THROW 'MT141-BLOCK-VIEW-OUTBOX'; };"
        } else {
            "REMOVE EVENT mt141_block_view_outbox_failpoint ON TABLE loom_block_view_fr_outbox;"
        })
        .await
    }

    pub async fn test_set_block_view_receipt_link_failpoint(
        &self,
        enabled: bool,
    ) -> StorageResult<()> {
        self.set_block_view_create_failpoint(if enabled {
            "DEFINE EVENT OVERWRITE mt141_block_view_receipt_link_failpoint ON TABLE loom_blocks \
             WHEN $event = 'UPDATE' AND $after.content_type = 'view_def' \
             THEN { THROW 'MT141-BLOCK-VIEW-RECEIPT-LINK'; };"
        } else {
            "REMOVE EVENT mt141_block_view_receipt_link_failpoint ON TABLE loom_blocks;"
        })
        .await
    }
}

pub(crate) async fn create_block_view(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    title: Option<String>,
    definition: BlockViewDefinition,
    metadata: MutationMetadata,
) -> StorageResult<BlockViewRecord> {
    if block_id.trim().is_empty() || block_id.trim() != block_id {
        return Err(StorageError::Validation(
            "loom block_id must be non-empty without surrounding whitespace",
        ));
    }
    require_resource(&metadata, block_id)?;
    let definition_json = encode_definition(&definition)?;
    let _guard = BLOCK_VIEW_MUTATION_LOCK.lock().await;

    if let Some(existing) = existing_view(db, block_id).await? {
        let existing_workspace = record_key(existing.workspace_id, WORKSPACES)?;
        if existing_workspace != workspace_id
            || existing.content_type != "view_def"
            || existing.title != title
            || existing.view_definition_json.as_deref() != Some(definition_json.as_str())
        {
            return Err(StorageError::Conflict("loom_block_view_id"));
        }
        let block = loom_store::get_loom_block(db, workspace_id, block_id).await?;
        return Ok(BlockViewRecord {
            block,
            definition: decode_definition(existing.view_definition_json.as_deref().ok_or(
                StorageError::Validation("view_def block missing definition"),
            )?)?,
            publication_event_id: prior_create_publication(db, workspace_id, block_id).await?,
        });
    }

    let flight_event = block_view_outbox::build_event(
        &metadata,
        workspace_id,
        block_id,
        BlockViewMutationOperation::Create,
    )?;
    let flight_event_json = serde_json::to_value(&flight_event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let flight_event_hash = event_hash(&flight_event)?;
    let (_, mutation_event) = event_ledger::prepare_event(mutation_event(
        workspace_id,
        block_id,
        "create_view_definition",
    )?)?;
    let entity_id = format!("KEN-{}", Uuid::now_v7().simple());
    let (_, bridge_event) =
        event_ledger::prepare_event(bridge_event(&metadata, workspace_id, block_id, &entity_id)?)?;
    let display_name = title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("view_def {block_id}"));
    let now = Datetime::from(metadata.timestamp);
    let content = BlockCreateContent {
        block_id: block_id.to_owned(),
        workspace_id: thing(WORKSPACES, workspace_id),
        content_type: "view_def".to_owned(),
        document_id: None,
        asset_id: None,
        title: title.clone(),
        original_filename: None,
        content_hash: None,
        pinned: false,
        favorite: false,
        pin_order: None,
        journal_date: None,
        last_job_id: metadata.job_id.map(|id| id.to_string()),
        last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
        last_actor_id: metadata.actor_id.clone(),
        edit_event_id: metadata.edit_event_id.to_string(),
        last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        created_at: now.clone(),
        updated_at: now.clone(),
        imported_at: None,
        backlink_count: 0,
        mention_count: 0,
        tag_count: 0,
        derived_json: serde_json::to_value(LoomBlockDerived::default())?,
        preview_status: PreviewStatus::None.as_str().to_owned(),
        thumbnail_asset_id: None,
        proxy_asset_id: None,
        view_definition_json: definition_json,
    };
    let rows = db
        .query_values_at::<MutationRow, _>(
            CREATE_TRANSACTION,
            CreateBindings {
                block: thing(BLOCKS, block_id),
                content,
                search: thing(SEARCH, block_id),
                workspace: thing(WORKSPACES, workspace_id),
                search_text: title.unwrap_or_default(),
                entity: thing(ENTITIES, entity_id.clone()),
                entity_id,
                display_name,
                detection_provenance: json!({
                    "extractor": "loom_block_knowledge_bridge",
                    "extractor_version": BRIDGE_EXTRACTOR_VERSION,
                    "method": "mt177_bridge",
                    "content_type": "view_def",
                }),
                bridge: thing(BRIDGES, block_id),
                bridge_event,
                mutation_event,
                outbox: thing(OUTBOX, flight_event.event_id.to_string()),
                outbox_content: OutboxContent {
                    event_id: flight_event.event_id.to_string(),
                    workspace_id: thing(WORKSPACES, workspace_id),
                    block_id: thing(BLOCKS, block_id),
                    operation: "create".to_owned(),
                    event: flight_event_json,
                    event_hash: flight_event_hash,
                    created_at: Datetime::from(flight_event.timestamp),
                },
            },
            8,
        )
        .await
        .map_err(map_err)?;
    if rows.first().map(|row| row.block_id.as_str()) != Some(block_id) {
        return Err(StorageError::Database(
            "loom block-view create returned no row".to_owned(),
        ));
    }
    let block = loom_store::get_loom_block(db, workspace_id, block_id).await?;
    Ok(BlockViewRecord {
        block,
        definition,
        publication_event_id: Some(flight_event.event_id),
    })
}

#[derive(SurrealValue)]
struct DefinitionRow {
    view_definition_json: Option<String>,
}

pub(crate) async fn get_block_view(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<BlockViewRecord> {
    let row: DefinitionRow = db
        .query_first(
            "SELECT view_definition_json FROM $record WHERE workspace_id = $workspace AND content_type = 'view_def' LIMIT 1;",
            WorkspaceRecordBinding {
                workspace: thing(WORKSPACES, workspace_id),
                record: thing(BLOCKS, block_id),
            },
        )
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("loom_block"))?;
    let definition = decode_definition(row.view_definition_json.as_deref().ok_or(
        StorageError::Validation("view_def block missing definition"),
    )?)?;
    Ok(BlockViewRecord {
        block: loom_store::get_loom_block(db, workspace_id, block_id).await?,
        definition,
        publication_event_id: None,
    })
}

#[derive(SurrealValue)]
struct UpdateBindings {
    block: RecordId,
    workspace: RecordId,
    definition_json: String,
    actor_kind: String,
    actor_id: Option<String>,
    job_id: Option<String>,
    workflow_id: Option<String>,
    edit_event_id: String,
    updated_at: Datetime,
    mutation_event: event_ledger::LedgerWrite,
    outbox: RecordId,
    outbox_content: OutboxContent,
}

const UPDATE_TRANSACTION: &str = "BEGIN TRANSACTION; \
    IF (SELECT VALUE id FROM $block WHERE workspace_id = $workspace AND content_type = 'view_def' LIMIT 1)[0] = NONE { THROW 'HSK-BLOCK-VIEW-NOT-FOUND'; }; \
    IF (SELECT VALUE id FROM kernel_event_ledger WHERE idempotency_key = $mutation_event.idempotency_key LIMIT 1)[0] = NONE { CREATE $mutation_event.record CONTENT { event_id: $mutation_event.event_id, event_version: $mutation_event.event_version, kernel_task_run_id: $mutation_event.kernel_task_run_id, session_run_id: $mutation_event.session_run_id, aggregate_type: $mutation_event.aggregate_type, aggregate_id: $mutation_event.aggregate_id, idempotency_key: $mutation_event.idempotency_key, event_type: $mutation_event.event_type, actor_kind: $mutation_event.actor_kind, actor_id: $mutation_event.actor_id, causation_id: $mutation_event.causation_id, correlation_id: $mutation_event.correlation_id, payload_hash: $mutation_event.payload_hash, source_component: $mutation_event.source_component, payload: $mutation_event.payload, created_at: $mutation_event.created_at }; }; \
    UPDATE $block SET view_definition_json = $definition_json, last_actor_kind = $actor_kind, last_actor_id = $actor_id, last_job_id = $job_id, last_workflow_id = $workflow_id, edit_event_id = $edit_event_id, updated_at = $updated_at, event_ledger_event_id = $mutation_event.record RETURN AFTER; \
    CREATE $outbox CONTENT $outbox_content; \
    COMMIT TRANSACTION;";

pub(crate) async fn update_block_view_definition(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    block_id: &str,
    definition: BlockViewDefinition,
    metadata: MutationMetadata,
) -> StorageResult<BlockViewRecord> {
    require_resource(&metadata, block_id)?;
    let definition_json = encode_definition(&definition)?;
    let _guard = BLOCK_VIEW_MUTATION_LOCK.lock().await;
    let flight_event = block_view_outbox::build_event(
        &metadata,
        workspace_id,
        block_id,
        BlockViewMutationOperation::Update,
    )?;
    let flight_event_json = serde_json::to_value(&flight_event)
        .map_err(|error| StorageError::Serialization(error.to_string()))?;
    let flight_event_hash = event_hash(&flight_event)?;
    let (_, mutation_event) = event_ledger::prepare_event(mutation_event(
        workspace_id,
        block_id,
        "update_view_definition",
    )?)?;
    let rows = db
        .query_values_at::<MutationRow, _>(
            UPDATE_TRANSACTION,
            UpdateBindings {
                block: thing(BLOCKS, block_id),
                workspace: thing(WORKSPACES, workspace_id),
                definition_json,
                actor_kind: metadata.actor_kind.as_str().to_owned(),
                actor_id: metadata.actor_id,
                job_id: metadata.job_id.map(|id| id.to_string()),
                workflow_id: metadata.workflow_id.map(|id| id.to_string()),
                edit_event_id: metadata.edit_event_id.to_string(),
                updated_at: Datetime::from(metadata.timestamp),
                mutation_event,
                outbox: thing(OUTBOX, flight_event.event_id.to_string()),
                outbox_content: OutboxContent {
                    event_id: flight_event.event_id.to_string(),
                    workspace_id: thing(WORKSPACES, workspace_id),
                    block_id: thing(BLOCKS, block_id),
                    operation: "update".to_owned(),
                    event: flight_event_json,
                    event_hash: flight_event_hash,
                    created_at: Datetime::from(flight_event.timestamp),
                },
            },
            3,
        )
        .await
        .map_err(map_err)?;
    if rows.first().map(|row| row.block_id.as_str()) != Some(block_id) {
        return Err(StorageError::NotFound("loom_block"));
    }
    Ok(BlockViewRecord {
        block: loom_store::get_loom_block(db, workspace_id, block_id).await?,
        definition,
        publication_event_id: Some(flight_event.event_id),
    })
}

#[derive(Clone, SurrealValue)]
struct PageBindings {
    workspace: RecordId,
    content_type: Option<String>,
    mime: Option<String>,
    date_from: Option<Datetime>,
    date_to: Option<Datetime>,
    journal_from: Option<String>,
    journal_to: Option<String>,
    tag_ids: Vec<RecordId>,
    mention_ids: Vec<RecordId>,
    limit: i64,
    offset: i64,
}

#[derive(SurrealValue)]
struct BlockIdRow {
    block_id: String,
}

macro_rules! page_query {
    ($order:literal) => {
        concat!(
            "SELECT block_id, title, created_at, updated_at, journal_date, content_type, pinned, favorite, backlink_count, mention_count, tag_count FROM loom_blocks WHERE workspace_id = $workspace ",
            "AND ($content_type = NONE OR content_type = $content_type) ",
            "AND ($mime = NONE OR (asset_id.workspace_id = $workspace AND asset_id.mime = $mime)) ",
            "AND ($date_from = NONE OR IF content_type = 'journal' AND journal_date != NONE { journal_date >= $journal_from } ELSE { updated_at >= $date_from }) ",
            "AND ($date_to = NONE OR IF content_type = 'journal' AND journal_date != NONE { journal_date <= $journal_to } ELSE { updated_at <= $date_to }) ",
            "AND (array::len($tag_ids) = 0 OR array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $parent.id AND edge_type = 'tag' AND target_block_id IN $tag_ids)) > 0) ",
            "AND (array::len($mention_ids) = 0 OR array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $parent.id AND edge_type = 'mention' AND target_block_id IN $mention_ids)) > 0) ",
            "ORDER BY ",
            $order,
            ", block_id ASC LIMIT $limit START $offset;"
        )
    };
}

macro_rules! nullable_count_query {
    ($field:literal) => {
        concat!(
            "SELECT count() AS count FROM loom_blocks WHERE workspace_id = $workspace ",
            "AND ($content_type = NONE OR content_type = $content_type) ",
            "AND ($mime = NONE OR (asset_id.workspace_id = $workspace AND asset_id.mime = $mime)) ",
            "AND ($date_from = NONE OR IF content_type = 'journal' AND journal_date != NONE { journal_date >= $journal_from } ELSE { updated_at >= $date_from }) ",
            "AND ($date_to = NONE OR IF content_type = 'journal' AND journal_date != NONE { journal_date <= $journal_to } ELSE { updated_at <= $date_to }) ",
            "AND (array::len($tag_ids) = 0 OR array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $parent.id AND edge_type = 'tag' AND target_block_id IN $tag_ids)) > 0) ",
            "AND (array::len($mention_ids) = 0 OR array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $parent.id AND edge_type = 'mention' AND target_block_id IN $mention_ids)) > 0) ",
            "AND ",
            $field,
            " != NONE GROUP ALL;"
        )
    };
}

macro_rules! nullable_page_query {
    ($field:literal, $predicate:literal, $order:literal) => {
        concat!(
            "SELECT block_id, title, created_at, updated_at, journal_date, content_type, pinned, favorite, backlink_count, mention_count, tag_count FROM loom_blocks WHERE workspace_id = $workspace ",
            "AND ($content_type = NONE OR content_type = $content_type) ",
            "AND ($mime = NONE OR (asset_id.workspace_id = $workspace AND asset_id.mime = $mime)) ",
            "AND ($date_from = NONE OR IF content_type = 'journal' AND journal_date != NONE { journal_date >= $journal_from } ELSE { updated_at >= $date_from }) ",
            "AND ($date_to = NONE OR IF content_type = 'journal' AND journal_date != NONE { journal_date <= $journal_to } ELSE { updated_at <= $date_to }) ",
            "AND (array::len($tag_ids) = 0 OR array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $parent.id AND edge_type = 'tag' AND target_block_id IN $tag_ids)) > 0) ",
            "AND (array::len($mention_ids) = 0 OR array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = $parent.id AND edge_type = 'mention' AND target_block_id IN $mention_ids)) > 0) ",
            "AND ",
            $field,
            " ",
            $predicate,
            " ORDER BY ",
            $order,
            ", block_id ASC LIMIT $limit START $offset;"
        )
    };
}

#[derive(SurrealValue)]
struct CountRow {
    count: i64,
}

async fn nullable_ascending_page(
    db: &SurrealDataContext<'_>,
    bindings: PageBindings,
    field: &BlockViewField,
) -> StorageResult<Vec<BlockIdRow>> {
    let (count_statement, present_statement, missing_statement) = match field {
        BlockViewField::Title => (
            nullable_count_query!("title"),
            nullable_page_query!("title", "!= NONE", "title ASC"),
            nullable_page_query!("title", "= NONE", "block_id ASC"),
        ),
        BlockViewField::JournalDate => (
            nullable_count_query!("journal_date"),
            nullable_page_query!("journal_date", "!= NONE", "journal_date ASC"),
            nullable_page_query!("journal_date", "= NONE", "block_id ASC"),
        ),
        _ => unreachable!("only nullable ascending fields use the split page"),
    };
    let present_count = db
        .query_first::<CountRow, _>(count_statement, bindings.clone())
        .await
        .map_err(map_err)?
        .map(|row| row.count)
        .unwrap_or_default()
        .max(0);
    let requested_limit = bindings.limit.max(0);
    let requested_offset = bindings.offset.max(0);
    let mut rows = Vec::new();
    if requested_offset < present_count && requested_limit > 0 {
        let mut present_bindings = bindings.clone();
        present_bindings.offset = requested_offset;
        present_bindings.limit = requested_limit.min(present_count - requested_offset);
        rows = db
            .query_values(present_statement, present_bindings)
            .await
            .map_err(map_err)?;
    }
    let remaining = requested_limit.saturating_sub(rows.len() as i64);
    if remaining > 0 {
        let mut missing_bindings = bindings;
        missing_bindings.limit = remaining;
        // Signed `saturating_sub` can still return a valid negative value (for example 0 - 3).
        // SurrealDB requires START to be non-negative, so clamp at the pagination boundary.
        missing_bindings.offset = requested_offset.saturating_sub(present_count).max(0);
        let mut missing: Vec<BlockIdRow> = db
            .query_values(missing_statement, missing_bindings)
            .await
            .map_err(map_err)?;
        rows.append(&mut missing);
    }
    Ok(rows)
}

fn page_statement(field: &BlockViewField, direction: BlockViewSortDirection) -> &'static str {
    match (field, direction) {
        (BlockViewField::Title, BlockViewSortDirection::Asc) => page_query!("title ASC"),
        (BlockViewField::Title, BlockViewSortDirection::Desc) => page_query!("title DESC"),
        (BlockViewField::Created, BlockViewSortDirection::Asc) => page_query!("created_at ASC"),
        (BlockViewField::Created, BlockViewSortDirection::Desc) => page_query!("created_at DESC"),
        (BlockViewField::Updated, BlockViewSortDirection::Asc) => page_query!("updated_at ASC"),
        (BlockViewField::Updated, BlockViewSortDirection::Desc) => page_query!("updated_at DESC"),
        (BlockViewField::JournalDate, BlockViewSortDirection::Asc) => {
            page_query!("journal_date ASC")
        }
        (BlockViewField::JournalDate, BlockViewSortDirection::Desc) => {
            page_query!("journal_date DESC")
        }
        (BlockViewField::ContentType, BlockViewSortDirection::Asc) => {
            page_query!("content_type ASC")
        }
        (BlockViewField::ContentType, BlockViewSortDirection::Desc) => {
            page_query!("content_type DESC")
        }
        (BlockViewField::Pinned, BlockViewSortDirection::Asc) => page_query!("pinned ASC"),
        (BlockViewField::Pinned, BlockViewSortDirection::Desc) => page_query!("pinned DESC"),
        (BlockViewField::Favorite, BlockViewSortDirection::Asc) => page_query!("favorite ASC"),
        (BlockViewField::Favorite, BlockViewSortDirection::Desc) => {
            page_query!("favorite DESC")
        }
        (BlockViewField::BacklinkCount, BlockViewSortDirection::Asc) => {
            page_query!("backlink_count ASC")
        }
        (BlockViewField::BacklinkCount, BlockViewSortDirection::Desc) => {
            page_query!("backlink_count DESC")
        }
        (BlockViewField::MentionCount, BlockViewSortDirection::Asc) => {
            page_query!("mention_count ASC")
        }
        (BlockViewField::MentionCount, BlockViewSortDirection::Desc) => {
            page_query!("mention_count DESC")
        }
        (BlockViewField::TagCount, BlockViewSortDirection::Asc) => page_query!("tag_count ASC"),
        (BlockViewField::TagCount, BlockViewSortDirection::Desc) => {
            page_query!("tag_count DESC")
        }
    }
}

#[derive(SurrealValue)]
struct EdgePageBindings {
    workspace: RecordId,
    blocks: Vec<RecordId>,
}

#[derive(SurrealValue)]
struct TagEdgeRow {
    source_block_id: RecordId,
    target_block_id: RecordId,
}

async fn partition_by_tag(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    blocks: &[LoomBlock],
    tag_ids: &[String],
) -> StorageResult<Vec<BlockViewLane>> {
    let mut tags_by_block: HashMap<String, HashSet<String>> = HashMap::new();
    if !blocks.is_empty() {
        let rows: Vec<TagEdgeRow> = db
            .query_values(
                "SELECT source_block_id, target_block_id FROM loom_edges WHERE workspace_id = $workspace \
                 AND edge_type = 'tag' AND source_block_id IN $blocks;",
                EdgePageBindings {
                    workspace: thing(WORKSPACES, workspace_id),
                    blocks: blocks
                        .iter()
                        .map(|block| thing(BLOCKS, block.block_id.clone()))
                        .collect(),
                },
            )
            .await
            .map_err(map_err)?;
        for row in rows {
            tags_by_block
                .entry(record_key(row.source_block_id, BLOCKS)?)
                .or_default()
                .insert(record_key(row.target_block_id, BLOCKS)?);
        }
    }

    let mut lane_keys = Vec::new();
    let mut seen = HashSet::new();
    for tag in tag_ids {
        if seen.insert(tag.clone()) {
            lane_keys.push(tag.clone());
        }
    }
    if tag_ids.is_empty() {
        for tags in tags_by_block.values() {
            for tag in tags {
                if seen.insert(tag.clone()) {
                    lane_keys.push(tag.clone());
                }
            }
        }
        lane_keys.sort();
    }

    let mut lanes: Vec<BlockViewLane> = lane_keys
        .into_iter()
        .map(|key| BlockViewLane {
            key,
            blocks: Vec::new(),
        })
        .collect();
    let mut untagged = BlockViewLane {
        key: BLOCK_VIEW_UNTAGGED_LANE.to_owned(),
        blocks: Vec::new(),
    };
    for block in blocks {
        let block_tags = tags_by_block.get(&block.block_id);
        let mut placed = false;
        for lane in &mut lanes {
            if block_tags.is_some_and(|tags| tags.contains(&lane.key)) {
                lane.blocks.push(block.clone());
                placed = true;
            }
        }
        if !placed {
            untagged.blocks.push(block.clone());
        }
    }
    lanes.push(untagged);
    Ok(lanes)
}

fn partition_by_field(blocks: &[LoomBlock], field: &BlockViewField) -> Vec<BlockViewLane> {
    let mut lanes: Vec<BlockViewLane> = Vec::new();
    for block in blocks {
        let key = match field {
            BlockViewField::ContentType => block.content_type.as_str().to_owned(),
            BlockViewField::Pinned => if block.pinned { "pinned" } else { "unpinned" }.to_owned(),
            BlockViewField::Favorite => if block.favorite {
                "favorite"
            } else {
                "not_favorite"
            }
            .to_owned(),
            BlockViewField::JournalDate => block
                .journal_date
                .clone()
                .unwrap_or_else(|| BLOCK_VIEW_UNTAGGED_LANE.to_owned()),
            BlockViewField::Title => block
                .title
                .clone()
                .unwrap_or_else(|| BLOCK_VIEW_UNTAGGED_LANE.to_owned()),
            BlockViewField::BacklinkCount => block.derived.backlink_count.to_string(),
            BlockViewField::MentionCount => block.derived.mention_count.to_string(),
            BlockViewField::TagCount => block.derived.tag_count.to_string(),
            BlockViewField::Created => block.created_at.to_rfc3339(),
            BlockViewField::Updated => block.updated_at.to_rfc3339(),
        };
        if let Some(lane) = lanes.iter_mut().find(|lane| lane.key == key) {
            lane.blocks.push(block.clone());
        } else {
            lanes.push(BlockViewLane {
                key,
                blocks: vec![block.clone()],
            });
        }
    }
    lanes
}

pub(crate) async fn query_block_view_results(
    db: &SurrealDataContext<'_>,
    workspace_id: &str,
    definition: &BlockViewDefinition,
    limit: u32,
    offset: u32,
) -> StorageResult<BlockViewResults> {
    let _ = encode_definition(definition)?;
    let filters = definition.query.to_filters();
    let (sort_field, sort_direction) = definition
        .sort
        .as_ref()
        .map(|sort| (&sort.field, sort.direction))
        .unwrap_or((&BlockViewField::Updated, BlockViewSortDirection::Desc));
    let journal_from = filters
        .date_from
        .as_ref()
        .map(|value| value.format("%Y-%m-%d").to_string());
    let journal_to = filters
        .date_to
        .as_ref()
        .map(|value| value.format("%Y-%m-%d").to_string());
    let bindings = PageBindings {
        workspace: thing(WORKSPACES, workspace_id),
        content_type: filters
            .content_type
            .as_ref()
            .map(LoomBlockContentType::as_str)
            .map(str::to_owned),
        mime: filters.mime,
        date_from: filters.date_from.map(Datetime::from),
        date_to: filters.date_to.map(Datetime::from),
        journal_from,
        journal_to,
        tag_ids: filters
            .tag_ids
            .iter()
            .map(|id| thing(BLOCKS, id.clone()))
            .collect(),
        mention_ids: filters
            .mention_ids
            .iter()
            .map(|id| thing(BLOCKS, id.clone()))
            .collect(),
        limit: i64::from(limit),
        offset: i64::from(offset),
    };
    let ids: Vec<BlockIdRow> = if sort_direction == BlockViewSortDirection::Asc
        && matches!(
            sort_field,
            BlockViewField::Title | BlockViewField::JournalDate
        ) {
        nullable_ascending_page(db, bindings, sort_field).await?
    } else {
        db.query_values(page_statement(sort_field, sort_direction), bindings)
            .await
            .map_err(map_err)?
    };
    let mut blocks = Vec::with_capacity(ids.len());
    for row in ids {
        blocks.push(loom_store::get_loom_block(db, workspace_id, &row.block_id).await?);
    }
    let total_returned = blocks.len() as u32;
    let groups = match (&definition.kind, &definition.group_by) {
        (BlockViewKind::Kanban, Some(BlockViewGroupBy::Tag)) => {
            partition_by_tag(db, workspace_id, &blocks, &definition.query.tag_ids).await?
        }
        (BlockViewKind::Kanban, Some(BlockViewGroupBy::Field { field })) => {
            partition_by_field(&blocks, field)
        }
        _ => Vec::new(),
    };
    Ok(BlockViewResults {
        kind: definition.kind,
        blocks,
        groups,
        total_returned,
    })
}
