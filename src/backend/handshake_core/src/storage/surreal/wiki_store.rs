//! Embedded SurrealDB storage for Loom wiki projections, overlays, markdown
//! import, and breadcrumb navigation.
//!
//! Projection rows remain disposable display content. Overlay annotations and
//! imported rich documents are authority, so their EventLedger receipts and
//! every derived row are committed in the same embedded transaction.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::str::FromStr;

use chrono::Utc;
use serde_json::{json, Value as JsonValue};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue};
use tokio::sync::Mutex;
use uuid::Uuid;

use super::{event_ledger, loom_store, SurrealDatabase, SurrealStorage, SurrealStorageError};
use crate::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use crate::storage::knowledge::{
    knowledge_canonical_json_sha256, new_knowledge_id, rich_document_loom_projection,
};
use crate::storage::{
    LoomBlock, LoomBlockContentType, LoomBlockDerived, LoomBreadcrumb, LoomBreadcrumbTrail,
    LoomMarkdownImport, LoomWikiOverlay, LoomWikiProjection, PreviewStatus, StorageError,
    StorageResult, WriteActorKind, WriteContext,
};

const WORKSPACES: &str = "workspaces";
const BLOCKS: &str = "loom_blocks";
const SEARCH_INDEX: &str = "loom_block_search_index";
const OVERLAYS: &str = "loom_wiki_overlays";
const RICH_DOCUMENTS: &str = "knowledge_rich_documents";
const ENTITIES: &str = "knowledge_entities";
const BRIDGES: &str = "loom_block_knowledge_bridge";
const BRIDGE_EXTRACTOR_VERSION: &str = "loom_block_knowledge_bridge_v1";

// The embedded database is single-process. These locks close the
// read/choose-identity/write races that the removed backend closed with
// transaction advisory locks.
static OVERLAY_MUTATION_LOCK: Mutex<()> = Mutex::const_new(());
static MARKDOWN_IMPORT_LOCK: Mutex<()> = Mutex::const_new(());
static WIKI_COMPILE_LOCK: Mutex<()> = Mutex::const_new(());

fn map_err(error: SurrealStorageError) -> StorageError {
    StorageError::Database(error.to_string())
}

fn thing(table: &str, key: impl Into<String>) -> RecordId {
    RecordId::new(table, key.into())
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

fn opt_record_key(
    record: Option<RecordId>,
    expected_table: &'static str,
) -> StorageResult<Option<String>> {
    record
        .map(|record| record_key(record, expected_table))
        .transpose()
}

#[derive(SurrealValue)]
struct WorkspaceProjectionBinding {
    workspace: RecordId,
    projection_id: String,
}

#[derive(SurrealValue)]
struct WorkspaceBlockBinding {
    workspace: RecordId,
    block: RecordId,
}

#[derive(SurrealValue)]
struct WorkspaceRow {
    name: String,
}

#[derive(SurrealValue)]
struct WorkspaceOnlyBinding {
    workspace: RecordId,
}

#[derive(SurrealValue)]
struct ProjectionRow {
    projection_id: String,
    workspace_id: RecordId,
    title: String,
    source_records: JsonValue,
    rendered_content: String,
    rebuild_status: String,
    staleness_hash: String,
    page_type: Option<String>,
    compile_stamp: Option<JsonValue>,
    page_links: JsonValue,
    created_at: Datetime,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CompileSourceSnapshot {
    block: RecordId,
    title: Option<String>,
    content_type: String,
    full_text_index: Option<String>,
    document_id: Option<RecordId>,
    asset_id: Option<RecordId>,
    content_hash: Option<String>,
    updated_at: Datetime,
}

#[derive(SurrealValue)]
struct CompileProjectionBinding {
    workspace: RecordId,
    title: String,
    projection_id: String,
    sources: Vec<CompileSourceSnapshot>,
    expected_ledger_version: i64,
    source_records: JsonValue,
    rendered_content: String,
    staleness_hash: String,
    compile_stamp: JsonValue,
    compile_recipe: JsonValue,
    page_links: JsonValue,
}

fn projection_to_domain(row: ProjectionRow) -> StorageResult<LoomWikiProjection> {
    let source_block_ids = row
        .source_records
        .as_array()
        .map(|records| {
            records
                .iter()
                .filter(|record| {
                    record.get("record_family").and_then(JsonValue::as_str) == Some("LoomBlock")
                })
                .filter_map(|record| record.get("record_id").and_then(JsonValue::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(LoomWikiProjection {
        projection_id: row.projection_id,
        workspace_id: record_key(row.workspace_id, WORKSPACES)?,
        title: row.title,
        source_block_ids,
        rendered_content: row.rendered_content,
        staleness_hash: row.staleness_hash,
        rebuild_status: row.rebuild_status,
        page_type: row.page_type,
        compile_stamp: row.compile_stamp,
        page_links: row.page_links,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct OverlayRow {
    overlay_id: String,
    projection_id: String,
    workspace_id: RecordId,
    annotation: String,
    anchor: Option<String>,
    created_at: Datetime,
    updated_at: Datetime,
}

fn overlay_to_domain(row: OverlayRow) -> StorageResult<LoomWikiOverlay> {
    Ok(LoomWikiOverlay {
        overlay_id: row.overlay_id,
        projection_id: row.projection_id,
        workspace_id: record_key(row.workspace_id, WORKSPACES)?,
        annotation: row.annotation,
        anchor: row.anchor,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
    })
}

#[derive(SurrealValue)]
struct OverlayLookupBinding {
    workspace: RecordId,
    overlay: RecordId,
}

#[derive(SurrealValue)]
struct OverlayProjectionRow {
    projection_id: String,
}

#[derive(SurrealValue)]
struct OverlayWriteBinding {
    workspace: RecordId,
    projection_id: String,
    overlay: RecordId,
    overlay_id: String,
    annotation: String,
    anchor: Option<String>,
    event: event_ledger::LedgerWrite,
}

#[derive(SurrealValue)]
struct OverlayDeleteBinding {
    workspace: RecordId,
    projection_id: String,
    overlay: RecordId,
    event: event_ledger::LedgerWrite,
}

/// Deterministic wiki markdown for a caller-ordered LoomBlock set.
fn render_loom_wiki_markdown(title: &str, blocks: &[LoomBlock]) -> String {
    let mut output = String::new();
    output.push_str("# ");
    output.push_str(title.trim());
    output.push('\n');
    if blocks.is_empty() {
        output.push_str("\n_No source blocks._\n");
        return output;
    }
    for block in blocks {
        let label = block
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id));
        output.push_str("\n## ");
        output.push_str(&label);
        output.push('\n');
        output.push_str("- type: ");
        output.push_str(block.content_type.as_str());
        output.push('\n');
        output.push_str("- cite: loom_block:");
        output.push_str(&block.block_id);
        output.push('\n');
        if let Some(text) = block
            .derived
            .full_text_index
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            output.push('\n');
            output.push_str(text);
            output.push('\n');
        }
    }
    output
}

fn loom_wiki_staleness_hash(blocks: &[LoomBlock]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"loom_wiki_projection_v1");
    for block in blocks {
        hasher.update(b"|id:");
        hasher.update(block.block_id.as_bytes());
        hasher.update(b"|t:");
        hasher.update(block.title.as_deref().unwrap_or("").as_bytes());
        hasher.update(b"|ct:");
        hasher.update(block.content_type.as_str().as_bytes());
        hasher.update(b"|ft:");
        hasher.update(
            block
                .derived
                .full_text_index
                .as_deref()
                .unwrap_or("")
                .as_bytes(),
        );
        hasher.update(b"|u:");
        hasher.update(block.updated_at.to_rfc3339().as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

async fn get_block(
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

pub(crate) async fn compile_loom_wiki_projection(
    database: &SurrealDatabase,
    workspace_id: &str,
    title: &str,
    block_ids: &[String],
) -> StorageResult<LoomWikiProjection> {
    use crate::knowledge_wiki::{
        loom_block_content_hash, CitedSource, CitedSourceKind, WikiCompileStamp,
    };

    let title = title.trim();
    if title.is_empty() {
        return Err(StorageError::Validation(
            "loom wiki projection title is required",
        ));
    }
    let mut blocks = Vec::with_capacity(block_ids.len());
    for block_id in block_ids {
        blocks.push(get_block(database.storage(), workspace_id, block_id).await?);
    }
    let rendered_content = render_loom_wiki_markdown(title, &blocks);
    let staleness_hash = loom_wiki_staleness_hash(&blocks);
    let source_records = json!(blocks
        .iter()
        .map(|block| json!({
            "record_family": "LoomBlock",
            "record_id": block.block_id,
            "content_hash": loom_block_content_hash(block),
        }))
        .collect::<Vec<_>>());
    let ledger_version = database.current_event_ledger_version().await?;
    let stamp = WikiCompileStamp::new(
        ledger_version,
        blocks
            .iter()
            .map(|block| CitedSource {
                kind: CitedSourceKind::LoomBlock,
                id: block.block_id.clone(),
                content_hash: loom_block_content_hash(block),
                span_id: None,
                source_id: None,
            })
            .collect(),
    );
    let sources = blocks
        .iter()
        .map(|block| CompileSourceSnapshot {
            block: thing(BLOCKS, block.block_id.clone()),
            title: block.title.clone(),
            content_type: block.content_type.as_str().to_owned(),
            full_text_index: block.derived.full_text_index.clone(),
            document_id: block
                .document_id
                .as_ref()
                .map(|id| thing("documents", id.clone())),
            asset_id: block
                .asset_id
                .as_ref()
                .map(|id| thing("assets", id.clone())),
            content_hash: block.content_hash.clone(),
            updated_at: Datetime::from(block.updated_at),
        })
        .collect();
    let bindings = CompileProjectionBinding {
        workspace: thing(WORKSPACES, workspace_id),
        title: title.to_owned(),
        projection_id: new_knowledge_id("KWP"),
        sources,
        expected_ledger_version: ledger_version,
        source_records,
        rendered_content,
        staleness_hash,
        compile_stamp: stamp.to_value(),
        compile_recipe: json!({
            "kind": "loom_topic",
            "block_ids": block_ids,
        }),
        page_links: json!([]),
    };

    // The initial reads may overlap concurrent Loom/EventLedger mutations.
    // Prove every content-bearing field and the ledger version against one
    // transactional snapshot, then publish the projection inside that same
    // transaction. A changed source fails closed instead of emitting a page
    // whose rendering, hashes, and compile stamp describe different moments.
    let _guard = WIKI_COMPILE_LOCK.lock().await;
    // Result-set index 6: BEGIN(0), source proof FOR(1), ledger LET(2),
    // ledger guard(3), stable-identity upsert IF(4), COMMIT(5), read(6).
    let rows = database
        .storage()
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<ProjectionRow, _>(
                        "BEGIN TRANSACTION; FOR $source IN $sources { LET $actual = (SELECT title, content_type, derived_json.full_text_index AS full_text_index, document_id, asset_id, content_hash, updated_at FROM ONLY $source.block WHERE workspace_id = $workspace); IF $actual = NONE OR $actual.title != $source.title OR $actual.content_type != $source.content_type OR $actual.full_text_index != $source.full_text_index OR $actual.document_id != $source.document_id OR $actual.asset_id != $source.asset_id OR $actual.content_hash != $source.content_hash OR $actual.updated_at != $source.updated_at { THROW 'HSK-LOOM-WIKI-SNAPSHOT-CHANGED'; }; }; LET $current_ledger_version = (SELECT VALUE event_sequence FROM kernel_event_ledger ORDER BY event_sequence DESC LIMIT 1)[0] ?? 0; IF $current_ledger_version != $expected_ledger_version { THROW 'HSK-LOOM-WIKI-LEDGER-CHANGED'; }; IF (SELECT VALUE id FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND title = $title LIMIT 1)[0] != NONE { RETURN UPDATE knowledge_wiki_projections SET source_records = $source_records, rendered_content = $rendered_content, rebuild_status = 'fresh', staleness_hash = $staleness_hash, rebuild_receipt_event_id = NONE, last_rebuilt_at = time::now(), page_type = NONE, compile_stamp = $compile_stamp, compile_recipe = $compile_recipe, page_links = $page_links, updated_at = time::now() WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND title = $title RETURN NONE; } ELSE { RETURN CREATE type::record('knowledge_wiki_projections', $projection_id) CONTENT { projection_id: $projection_id, workspace_id: $workspace, projection_kind: 'wiki_page', title: $title, source_records: $source_records, rendered_content: $rendered_content, rebuild_status: 'fresh', staleness_hash: $staleness_hash, rebuild_receipt_event_id: NONE, last_rebuilt_at: time::now(), page_type: NONE, compile_stamp: $compile_stamp, compile_recipe: $compile_recipe, page_links: $page_links } RETURN NONE; }; COMMIT TRANSACTION; SELECT projection_id, workspace_id, title, source_records, rendered_content, rebuild_status, staleness_hash, page_type, compile_stamp, page_links, created_at, updated_at FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_kind = 'wiki_page' AND title = $title LIMIT 1;",
                        bindings,
                        6,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| {
            let rendered = error.to_string();
            if rendered.contains("HSK-LOOM-WIKI-SNAPSHOT-CHANGED") {
                StorageError::Conflict("loom wiki source changed during compilation")
            } else if rendered.contains("HSK-LOOM-WIKI-LEDGER-CHANGED") {
                StorageError::Conflict("EventLedger changed during loom wiki compilation")
            } else {
                map_err(error)
            }
        })?;
    rows.into_iter()
        .next()
        .ok_or_else(|| {
            StorageError::Database("loom wiki projection compile returned no page".to_owned())
        })
        .and_then(projection_to_domain)
}

pub(crate) async fn get_loom_wiki_projection(
    storage: &SurrealStorage,
    workspace_id: &str,
    projection_id: &str,
) -> StorageResult<LoomWikiProjection> {
    let row = storage
        .with_data_operation({
            let bindings = WorkspaceProjectionBinding {
                workspace: thing(WORKSPACES, workspace_id),
                projection_id: projection_id.to_owned(),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first::<ProjectionRow, _>(
                            "SELECT projection_id, workspace_id, title, source_records, rendered_content, rebuild_status, staleness_hash, page_type, compile_stamp, page_links, created_at, updated_at FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_id = $projection_id LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("loom_wiki_projection"))?;
    projection_to_domain(row)
}

pub(crate) async fn loom_wiki_projection_is_stale(
    storage: &SurrealStorage,
    workspace_id: &str,
    projection_id: &str,
) -> StorageResult<bool> {
    let projection = get_loom_wiki_projection(storage, workspace_id, projection_id).await?;
    if projection.page_type.is_some() {
        return Err(StorageError::Validation(
            "typed project-wiki pages take staleness from the drift verdict",
        ));
    }
    let mut blocks = Vec::with_capacity(projection.source_block_ids.len());
    for block_id in &projection.source_block_ids {
        match get_block(storage, workspace_id, block_id).await {
            Ok(block) => blocks.push(block),
            Err(StorageError::NotFound(_)) => return Ok(true),
            Err(error) => return Err(error),
        }
    }
    Ok(loom_wiki_staleness_hash(&blocks) != projection.staleness_hash)
}

pub(crate) async fn regenerate_loom_wiki_projection(
    database: &SurrealDatabase,
    workspace_id: &str,
    projection_id: &str,
) -> StorageResult<LoomWikiProjection> {
    let current = get_loom_wiki_projection(database.storage(), workspace_id, projection_id).await?;
    if current.page_type.is_some() {
        return Err(StorageError::Validation(
            "typed project-wiki pages regenerate via the project wiki engine",
        ));
    }
    let mut surviving_ids = Vec::new();
    for block_id in &current.source_block_ids {
        match get_block(database.storage(), workspace_id, block_id).await {
            Ok(block) => surviving_ids.push(block.block_id),
            Err(StorageError::NotFound(_)) => {}
            Err(error) => return Err(error),
        }
    }
    compile_loom_wiki_projection(database, workspace_id, &current.title, &surviving_ids).await
}

pub(crate) async fn delete_loom_wiki_projection(
    storage: &SurrealStorage,
    workspace_id: &str,
    projection_id: &str,
) -> StorageResult<()> {
    let deleted = storage
        .with_data_operation({
            let bindings = WorkspaceProjectionBinding {
                workspace: thing(WORKSPACES, workspace_id),
                projection_id: projection_id.to_owned(),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values::<ProjectionRow, _>(
                            "DELETE knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_id = $projection_id RETURN BEFORE;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    if deleted.is_empty() {
        return Err(StorageError::NotFound("loom_wiki_projection"));
    }
    // `loom_wiki_overlays.projection_id` is deliberately a string, not a
    // cascading record link: operator-authored overlays survive projection
    // churn and can be reattached when the page is regenerated.
    Ok(())
}

fn wiki_mutation_event(
    workspace_id: &str,
    projection_id: &str,
    overlay_id: &str,
    operation: &str,
    detail: JsonValue,
) -> StorageResult<event_ledger::LedgerWrite> {
    let run_id = format!("LOOM-WIKI-{workspace_id}");
    let mut payload = json!({
        "type": "knowledge_loom_wiki_mutated",
        "schema_id": "hsk.loom_wiki_mutation@1",
        "workspace_id": workspace_id,
        "projection_id": projection_id,
        "overlay_id": overlay_id,
        "operation": operation,
    });
    if let (JsonValue::Object(target), JsonValue::Object(detail)) = (&mut payload, detail) {
        target.extend(detail);
    }
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomWikiMutated,
        KernelActor::System("loom-wiki".to_owned()),
    )
    .aggregate("loom_wiki_overlay", overlay_id.to_owned())
    .source_component("loom_wiki")
    .payload(payload)
    .build()
    .map_err(|_| StorageError::Validation("loom wiki EventLedger receipt build failed"))?;
    event_ledger::prepare_event(event).map(|(_, write)| write)
}

pub(crate) async fn add_loom_wiki_overlay(
    storage: &SurrealStorage,
    workspace_id: &str,
    projection_id: &str,
    annotation: &str,
    anchor: Option<&str>,
) -> StorageResult<LoomWikiOverlay> {
    let annotation = annotation.trim();
    if annotation.is_empty() {
        return Err(StorageError::Validation(
            "loom wiki overlay annotation is required",
        ));
    }
    let anchor = anchor
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    let overlay_id = format!("LWO-{}", Uuid::now_v7().simple());
    let event = wiki_mutation_event(
        workspace_id,
        projection_id,
        &overlay_id,
        "overlay_added",
        json!({ "has_anchor": anchor.is_some() }),
    )?;
    let _guard = OVERLAY_MUTATION_LOCK.lock().await;
    let rows = storage
        .with_data_operation({
            let bindings = OverlayWriteBinding {
                workspace: thing(WORKSPACES, workspace_id),
                projection_id: projection_id.to_owned(),
                overlay: thing(OVERLAYS, overlay_id.clone()),
                overlay_id,
                annotation: annotation.to_owned(),
                anchor,
                event,
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at::<OverlayRow, _>(
                            "BEGIN TRANSACTION; IF (SELECT VALUE id FROM knowledge_wiki_projections WHERE workspace_id = $workspace AND projection_id = $projection_id LIMIT 1)[0] = NONE { THROW 'HSK-LOOM-WIKI-PROJECTION-NOT-FOUND'; }; CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; CREATE $overlay CONTENT { overlay_id: $overlay_id, projection_id: $projection_id, workspace_id: $workspace, annotation: $annotation, anchor: $anchor, event_ledger_event_id: $event.record } RETURN AFTER; COMMIT TRANSACTION;",
                            bindings,
                            3,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(|error| {
            if error
                .to_string()
                .contains("HSK-LOOM-WIKI-PROJECTION-NOT-FOUND")
            {
                StorageError::NotFound("loom_wiki_projection")
            } else {
                map_err(error)
            }
        })?;
    rows.into_iter()
        .next()
        .ok_or_else(|| {
            StorageError::Database("loom wiki overlay create returned no row".to_owned())
        })
        .and_then(overlay_to_domain)
}

pub(crate) async fn list_loom_wiki_overlays(
    storage: &SurrealStorage,
    workspace_id: &str,
    projection_id: &str,
) -> StorageResult<Vec<LoomWikiOverlay>> {
    let rows = storage
        .with_data_operation({
            let bindings = WorkspaceProjectionBinding {
                workspace: thing(WORKSPACES, workspace_id),
                projection_id: projection_id.to_owned(),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values::<OverlayRow, _>(
                            "SELECT overlay_id, projection_id, workspace_id, annotation, anchor, created_at, updated_at FROM loom_wiki_overlays WHERE workspace_id = $workspace AND projection_id = $projection_id ORDER BY created_at ASC, overlay_id ASC;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    rows.into_iter().map(overlay_to_domain).collect()
}

pub(crate) async fn delete_loom_wiki_overlay(
    storage: &SurrealStorage,
    workspace_id: &str,
    overlay_id: &str,
) -> StorageResult<()> {
    let _guard = OVERLAY_MUTATION_LOCK.lock().await;
    let projection = storage
        .with_data_operation({
            let bindings = OverlayLookupBinding {
                workspace: thing(WORKSPACES, workspace_id),
                overlay: thing(OVERLAYS, overlay_id),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first::<OverlayProjectionRow, _>(
                            "SELECT projection_id FROM $overlay WHERE workspace_id = $workspace LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?
        .ok_or(StorageError::NotFound("loom_wiki_overlay"))?;
    let event = wiki_mutation_event(
        workspace_id,
        &projection.projection_id,
        overlay_id,
        "overlay_deleted",
        json!({}),
    )?;
    let deleted = storage
        .with_data_operation({
            let bindings = OverlayDeleteBinding {
                workspace: thing(WORKSPACES, workspace_id),
                projection_id: projection.projection_id,
                overlay: thing(OVERLAYS, overlay_id),
                event,
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values_at::<OverlayRow, _>(
                            "BEGIN TRANSACTION; IF (SELECT VALUE id FROM $overlay WHERE workspace_id = $workspace AND projection_id = $projection_id)[0] = NONE { THROW 'HSK-LOOM-WIKI-OVERLAY-NOT-FOUND'; }; CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at }; DELETE $overlay RETURN BEFORE; COMMIT TRANSACTION;",
                            bindings,
                            3,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(|error| {
            if error
                .to_string()
                .contains("HSK-LOOM-WIKI-OVERLAY-NOT-FOUND")
            {
                StorageError::NotFound("loom_wiki_overlay")
            } else {
                map_err(error)
            }
        })?;
    if deleted.is_empty() {
        return Err(StorageError::NotFound("loom_wiki_overlay"));
    }
    Ok(())
}

#[derive(SurrealValue)]
struct RichDocumentCandidateRow {
    rich_document_id: String,
    title: String,
    is_live: bool,
}

#[derive(SurrealValue)]
struct LoomCandidateRow {
    block_id: String,
    workspace_id: RecordId,
}

#[derive(Clone)]
struct ResolvedBacklink {
    backlink_id: String,
    relationship_id: String,
    link_kind: String,
    target: String,
    block_id: String,
    project_to_loom: bool,
}

#[derive(SurrealValue)]
struct CandidateBinding {
    workspace: RecordId,
    candidate_ids: Vec<String>,
    candidate_titles: Vec<String>,
}

#[derive(SurrealValue)]
struct LoomCandidateBinding {
    candidate_ids: Vec<String>,
}

async fn resolve_initial_backlinks(
    storage: &SurrealStorage,
    workspace_id: &str,
    rich_document_id: &str,
    schema_version: &str,
    content_json: &JsonValue,
) -> StorageResult<Vec<ResolvedBacklink>> {
    use crate::knowledge_document::backlink::DocumentLinkReferences;
    use crate::knowledge_document::block_tree::BlockTree;

    let tree = BlockTree::from_document_json(rich_document_id, schema_version, content_json)
        .map_err(|_| StorageError::Validation("knowledge rich document block tree is malformed"))?;
    let references = DocumentLinkReferences::extract(&tree).references;
    let mut candidate_titles: Vec<String> = references
        .iter()
        .filter(|reference| {
            reference.kind.as_str() == "wikilink" && !reference.target.starts_with("KRD-")
        })
        .map(|reference| reference.target.clone())
        .collect();
    candidate_titles.sort();
    candidate_titles.dedup();
    let mut candidate_ids: Vec<String> = references
        .iter()
        .filter(|reference| reference.target.starts_with("KRD-"))
        .map(|reference| reference.target.clone())
        .collect();
    candidate_ids.sort();
    candidate_ids.dedup();

    let document_candidates = storage
        .with_data_operation({
            let bindings = CandidateBinding {
                workspace: thing(WORKSPACES, workspace_id),
                candidate_ids,
                candidate_titles,
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_values::<RichDocumentCandidateRow, _>(
                            "SELECT rich_document_id, title, (deleted_at = NONE) AS is_live FROM knowledge_rich_documents WHERE workspace_id = $workspace AND (rich_document_id IN $candidate_ids OR title IN $candidate_titles) ORDER BY rich_document_id ASC;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    let live_ids: HashSet<String> = document_candidates
        .iter()
        .filter(|row| row.is_live)
        .map(|row| row.rich_document_id.clone())
        .collect();
    let deleted_titles: HashSet<String> = document_candidates
        .iter()
        .filter(|row| !row.is_live)
        .map(|row| row.title.clone())
        .collect();
    let mut live_ids_by_title: HashMap<String, Vec<String>> = HashMap::new();
    for row in document_candidates.into_iter().filter(|row| row.is_live) {
        live_ids_by_title
            .entry(row.title)
            .or_default()
            .push(row.rich_document_id);
    }

    let mut loom_candidate_ids: Vec<String> = references
        .iter()
        .filter(|reference| reference.kind.as_str() == "wikilink")
        .map(|reference| reference.target.clone())
        .chain(live_ids.iter().cloned())
        .collect();
    loom_candidate_ids.sort();
    loom_candidate_ids.dedup();
    let loom_candidates = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values::<LoomCandidateRow, _>(
                        "SELECT block_id, workspace_id FROM loom_blocks WHERE block_id IN $candidate_ids ORDER BY block_id ASC;",
                        LoomCandidateBinding {
                            candidate_ids: loom_candidate_ids,
                        },
                    )
                    .await
            })
        })
        .await
        .map_err(map_err)?;
    let mut live_loom_ids = HashSet::new();
    let mut foreign_loom_ids = HashSet::new();
    for row in loom_candidates {
        if record_key(row.workspace_id, WORKSPACES)? == workspace_id {
            live_loom_ids.insert(row.block_id);
        } else {
            foreign_loom_ids.insert(row.block_id);
        }
    }

    let mut resolved = Vec::with_capacity(references.len());
    for reference in references {
        let link_kind = reference.kind.as_str().to_owned();
        let target = if link_kind == "wikilink" && live_loom_ids.contains(&reference.target) {
            reference.target.clone()
        } else if link_kind == "wikilink" && foreign_loom_ids.contains(&reference.target) {
            continue;
        } else if link_kind == "wikilink" && reference.target.starts_with("KRD-") {
            if !live_ids.contains(&reference.target) {
                continue;
            }
            reference.target.clone()
        } else if link_kind == "wikilink" {
            match live_ids_by_title.get(&reference.target) {
                Some(matches) if matches.len() == 1 => matches[0].clone(),
                Some(_) => reference.target.clone(),
                None if deleted_titles.contains(&reference.target) => continue,
                None => reference.target.clone(),
            }
        } else {
            reference.target.clone()
        };
        if link_kind == "wikilink" && live_ids.contains(&target) && !live_loom_ids.contains(&target)
        {
            return Err(StorageError::Conflict(
                "knowledge backlink target is missing its LoomBlock projection",
            ));
        }
        resolved.push(ResolvedBacklink {
            backlink_id: new_knowledge_id("KDBL"),
            relationship_id: reference.relationship_id,
            link_kind,
            project_to_loom: live_loom_ids.contains(&target),
            target,
            block_id: reference.block_id,
        });
    }
    Ok(resolved)
}

#[derive(SurrealValue)]
struct BlockRow {
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
}

fn block_to_domain(row: BlockRow) -> StorageResult<LoomBlock> {
    let mut derived: LoomBlockDerived =
        serde_json::from_value(row.derived_json).unwrap_or_default();
    derived.backlink_count = row.backlink_count;
    derived.mention_count = row.mention_count;
    derived.tag_count = row.tag_count;
    derived.preview_status = PreviewStatus::from_str(&row.preview_status)?;
    derived.thumbnail_asset_id = opt_record_key(row.thumbnail_asset_id, "assets")?;
    derived.proxy_asset_id = opt_record_key(row.proxy_asset_id, "assets")?;
    Ok(LoomBlock {
        block_id: row.block_id,
        workspace_id: record_key(row.workspace_id, WORKSPACES)?,
        content_type: LoomBlockContentType::from_str(&row.content_type)?,
        document_id: opt_record_key(row.document_id, "documents")?,
        asset_id: opt_record_key(row.asset_id, "assets")?,
        title: row.title,
        original_filename: row.original_filename,
        content_hash: row.content_hash,
        pinned: row.pinned,
        favorite: row.favorite,
        pin_order: row
            .pin_order
            .map(i32::try_from)
            .transpose()
            .map_err(|_| StorageError::Serialization("pin_order exceeds i32".to_owned()))?,
        journal_date: row.journal_date,
        created_at: row.created_at.into_inner(),
        updated_at: row.updated_at.into_inner(),
        imported_at: row.imported_at.map(Datetime::into_inner),
        derived,
    })
}

#[derive(SurrealValue)]
struct MarkdownImportBinding {
    workspace: RecordId,
    doc: RecordId,
    block: RecordId,
    doc_id: String,
    title: String,
    schema_version: String,
    content_json: JsonValue,
    content_sha256: String,
    derived_json: JsonValue,
    search: RecordId,
    search_text: String,
    last_job_id: Option<String>,
    last_workflow_id: Option<String>,
    last_actor_id: Option<String>,
    last_actor_kind: String,
    edit_event_id: String,
    written_at: Datetime,
    backlink_rows: JsonValue,
    loom_edge_rows: JsonValue,
    affected_blocks: Vec<String>,
    entity: RecordId,
    entity_id: String,
    bridge: RecordId,
    detection_provenance: JsonValue,
    event: event_ledger::LedgerWrite,
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

pub(crate) async fn import_markdown_to_loom(
    storage: &SurrealStorage,
    ctx: &WriteContext,
    workspace_id: &str,
    title: &str,
    markdown: &str,
) -> StorageResult<LoomMarkdownImport> {
    use crate::knowledge_document::block_tree::DOCUMENT_SCHEMA_VERSION;
    use crate::knowledge_document::import::{import_snippet, ImportFormat};

    let title = title.trim();
    if title.is_empty() {
        return Err(StorageError::Validation("loom import title is required"));
    }
    let outcome = import_snippet(markdown, ImportFormat::Markdown);
    let warnings = outcome
        .warnings
        .iter()
        .map(|warning| format!("{}: {}", warning.code, warning.detail))
        .collect();
    let rich_document_id = new_knowledge_id("KRD");
    let metadata = storage
        .inner
        .guard
        .validate_write(ctx, &rich_document_id)
        .await
        .map_err(StorageError::from)?;
    let content_sha256 = knowledge_canonical_json_sha256(&outcome.document_json);
    let (derived_json, search_text) = rich_document_loom_projection(title, &outcome.document_json)?;
    let derived_json: JsonValue = serde_json::from_str(&derived_json)?;

    let _guard = MARKDOWN_IMPORT_LOCK.lock().await;
    let resolved = resolve_initial_backlinks(
        storage,
        workspace_id,
        &rich_document_id,
        DOCUMENT_SCHEMA_VERSION,
        &outcome.document_json,
    )
    .await?;
    let backlink_rows = JsonValue::Array(
        resolved
            .iter()
            .map(|row| {
                json!({
                    "backlink_id": row.backlink_id,
                    "relationship_id": row.relationship_id,
                    "link_kind": row.link_kind,
                    "target": row.target,
                    "block_id": row.block_id,
                })
            })
            .collect(),
    );
    let loom_edge_rows = JsonValue::Array(
        resolved
            .iter()
            .filter(|row| row.project_to_loom)
            .map(|row| {
                json!({
                    "relationship_id": row.relationship_id,
                    "target": row.target,
                    "block_id": row.block_id,
                })
            })
            .collect(),
    );
    let affected_blocks: Vec<String> = resolved
        .iter()
        .filter(|row| row.project_to_loom)
        .map(|row| row.target.clone())
        .chain(std::iter::once(rich_document_id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let entity_id = format!("KEN-{}", Uuid::now_v7().simple());
    let run_id = format!("LOOM-BRIDGE-{workspace_id}");
    let event = NewKernelEvent::builder(
        run_id.clone(),
        run_id,
        KernelEventType::KnowledgeLoomBlockIndexed,
        bridge_actor(ctx),
    )
    .aggregate("knowledge_loom_block", entity_id.clone())
    .idempotency_key(format!(
        "KEI-loom-bridge-{}-{}",
        entity_id,
        Utc::now().timestamp_nanos_opt().unwrap_or_default()
    ))
    .source_component("loom_block_knowledge_bridge")
    .payload(json!({
        "type": "knowledge_loom_block_indexed",
        "workspace_id": workspace_id,
        "block_id": rich_document_id.clone(),
        "entity_id": entity_id.clone(),
        "content_type": "note",
        "extractor_version": BRIDGE_EXTRACTOR_VERSION,
    }))
    .build()
    .map_err(|_| StorageError::Validation("loom bridge EventLedger receipt build failed"))?;
    let (_, event) = event_ledger::prepare_event(event)?;
    let bindings = MarkdownImportBinding {
        workspace: thing(WORKSPACES, workspace_id),
        doc: thing(RICH_DOCUMENTS, rich_document_id.clone()),
        block: thing(BLOCKS, rich_document_id.clone()),
        doc_id: rich_document_id.clone(),
        title: title.to_owned(),
        schema_version: DOCUMENT_SCHEMA_VERSION.to_owned(),
        content_json: outcome.document_json,
        content_sha256,
        derived_json,
        search: thing(SEARCH_INDEX, rich_document_id.clone()),
        search_text,
        last_job_id: metadata.job_id.map(|id| id.to_string()),
        last_workflow_id: metadata.workflow_id.map(|id| id.to_string()),
        last_actor_id: metadata.actor_id,
        last_actor_kind: metadata.actor_kind.as_str().to_owned(),
        edit_event_id: metadata.edit_event_id.to_string(),
        written_at: Datetime::from(metadata.timestamp),
        backlink_rows,
        loom_edge_rows,
        affected_blocks,
        entity: thing(ENTITIES, entity_id.clone()),
        entity_id,
        bridge: thing(BRIDGES, rich_document_id.clone()),
        detection_provenance: json!({
            "extractor": "loom_block_knowledge_bridge",
            "extractor_version": BRIDGE_EXTRACTOR_VERSION,
            "method": "mt177_bridge",
            "content_type": "note",
        }),
        event,
    };
    // Result-set index 13: BEGIN(0), workspace guard(1), document(2),
    // LoomBlock(3), search(4), version(5), backlinks(6), Loom edges(7),
    // metrics(8), entity(9), EventLedger(10), bridge(11), COMMIT(12), read(13).
    let rows = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database
                    .query_values_at::<BlockRow, _>(
                        "BEGIN TRANSACTION; IF !record::exists($workspace) { THROW 'HSK-LOOM-IMPORT-WORKSPACE-NOT-FOUND'; }; CREATE $doc CONTENT { rich_document_id: $doc_id, workspace_id: $workspace, title: $title, schema_version: $schema_version, content_json: $content_json, content_sha256: $content_sha256, authority_label: 'promoted', created_at: $written_at, updated_at: $written_at } RETURN NONE; CREATE $block CONTENT { block_id: $doc_id, workspace_id: $workspace, content_type: 'note', title: $title, content_hash: $content_sha256, derived_json: $derived_json, last_job_id: $last_job_id, last_workflow_id: $last_workflow_id, last_actor_id: $last_actor_id, last_actor_kind: $last_actor_kind, edit_event_id: $edit_event_id, created_at: $written_at, updated_at: $written_at } RETURN NONE; CREATE $search CONTENT { block_id: $block, workspace_id: $workspace, content_type: 'note', search_text: $search_text, indexed_at: $written_at } RETURN NONE; CREATE knowledge_rich_document_versions CONTENT { rich_document_id: $doc, doc_version: 1, schema_version: $schema_version, content_json: $content_json, content_sha256: $content_sha256, created_at: $written_at } RETURN NONE; FOR $row IN $backlink_rows { CREATE type::record('knowledge_document_backlinks', $row.backlink_id) CONTENT { backlink_id: $row.backlink_id, workspace_id: $workspace, relationship_id: $row.relationship_id, source_document_id: $doc, link_kind: $row.link_kind, target: $row.target, block_id: $row.block_id } RETURN NONE; }; FOR $row IN $loom_edge_rows { IF (SELECT VALUE id FROM loom_edges WHERE edge_id = $row.relationship_id LIMIT 1)[0] != NONE { THROW 'HSK-KDBL-LOOM-EDGE-OWNED'; }; CREATE loom_edges CONTENT { edge_id: $row.relationship_id, workspace_id: $workspace, source_block_id: $block, target_block_id: type::record('loom_blocks', $row.target), edge_type: 'mention', created_by: 'user', last_actor_kind: 'SYSTEM', last_actor_id: 'knowledge_rich_document_backlink_projection', edit_event_id: '00000000-0000-0000-0000-000000000000', source_document_id: $doc_id, source_text_block_id: $row.block_id } RETURN NONE; }; FOR $affected IN $affected_blocks { UPDATE loom_blocks SET mention_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $affected) AND edge_type = 'mention')), tag_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND source_block_id = type::record('loom_blocks', $affected) AND edge_type = 'tag')), backlink_count = array::len((SELECT VALUE id FROM loom_edges WHERE workspace_id = $workspace AND target_block_id = type::record('loom_blocks', $affected) AND edge_type IN ['mention', 'tag'])) WHERE workspace_id = $workspace AND block_id = $affected RETURN NONE; }; CREATE $entity CONTENT { entity_id: $entity_id, workspace_id: $workspace, entity_kind: 'loom_block', entity_key: $doc_id, display_name: $title, detection_provenance: $detection_provenance, lifecycle_state: 'active', created_at: $written_at, updated_at: $written_at } RETURN NONE; CREATE $event.record CONTENT { event_id: $event.event_id, event_version: $event.event_version, kernel_task_run_id: $event.kernel_task_run_id, session_run_id: $event.session_run_id, aggregate_type: $event.aggregate_type, aggregate_id: $event.aggregate_id, idempotency_key: $event.idempotency_key, event_type: $event.event_type, actor_kind: $event.actor_kind, actor_id: $event.actor_id, causation_id: $event.causation_id, correlation_id: $event.correlation_id, payload_hash: $event.payload_hash, source_component: $event.source_component, payload: $event.payload, created_at: $event.created_at } RETURN NONE; CREATE $bridge CONTENT { block_id: $block, workspace_id: $workspace, entity_id: $entity, index_event_id: $event.record, created_at: $written_at, updated_at: $written_at } RETURN NONE; COMMIT TRANSACTION; SELECT * FROM $block;",
                        bindings,
                        13,
                    )
                    .await
            })
        })
        .await
        .map_err(|error| {
            let rendered = error.to_string();
            if rendered.contains("HSK-LOOM-IMPORT-WORKSPACE-NOT-FOUND") {
                StorageError::NotFound("workspace")
            } else if rendered.contains("HSK-KDBL-LOOM-EDGE-OWNED") {
                StorageError::Conflict(
                    "knowledge backlink Loom edge identity is owned by another writer",
                )
            } else {
                map_err(error)
            }
        })?;
    let block = rows
        .into_iter()
        .next()
        .ok_or_else(|| StorageError::Database("loom markdown import returned no block".to_owned()))
        .and_then(block_to_domain)?;
    Ok(LoomMarkdownImport {
        block,
        rich_document_id,
        warnings,
    })
}

#[derive(SurrealValue)]
struct FirstFolderRow {
    folder_id: String,
    parent_folder_id: Option<RecordId>,
    name: String,
    project_ref: Option<String>,
    created_at: Datetime,
}

#[derive(SurrealValue)]
struct FolderLookupBinding {
    workspace: RecordId,
    block: RecordId,
}

#[derive(SurrealValue)]
struct FolderRecordBinding {
    workspace: RecordId,
    folder: RecordId,
}

#[derive(SurrealValue)]
struct BridgeEntityRow {
    entity_id: RecordId,
}

pub(crate) async fn loom_block_breadcrumbs(
    storage: &SurrealStorage,
    workspace_id: &str,
    block_id: &str,
) -> StorageResult<LoomBreadcrumbTrail> {
    let block = get_block(storage, workspace_id, block_id).await?;
    let workspace = storage
        .with_data_operation({
            let workspace = WorkspaceOnlyBinding {
                workspace: thing(WORKSPACES, workspace_id),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first::<WorkspaceRow, _>(
                            "SELECT name FROM $workspace LIMIT 1;",
                            workspace,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    let mut crumbs = vec![LoomBreadcrumb {
        kind: "workspace".to_owned(),
        id: workspace_id.to_owned(),
        label: workspace
            .map(|row| row.name)
            .unwrap_or_else(|| workspace_id.to_owned()),
    }];

    let first_folder = storage
        .with_data_operation({
            let bindings = FolderLookupBinding {
                workspace: thing(WORKSPACES, workspace_id),
                block: thing(BLOCKS, block_id),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first::<FirstFolderRow, _>(
                            "SELECT folder_id, parent_folder_id, name, project_ref, created_at FROM loom_folders WHERE workspace_id = $workspace AND id IN (SELECT VALUE folder_id FROM loom_folder_members WHERE workspace_id = $workspace AND block_id = $block) ORDER BY created_at ASC, folder_id ASC LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    if let Some(folder) = first_folder {
        let mut ancestry = Vec::new();
        let mut current = Some(folder);
        let mut seen = HashSet::new();
        while let Some(folder) = current {
            if !seen.insert(folder.folder_id.clone()) {
                break;
            }
            let parent = folder.parent_folder_id.clone();
            ancestry.push(folder);
            current = match parent {
                None => None,
                Some(parent) => {
                    let parent_id = record_key(parent, "loom_folders")?;
                    storage
                        .with_data_operation({
                            let bindings = FolderRecordBinding {
                                workspace: thing(WORKSPACES, workspace_id),
                                folder: thing("loom_folders", parent_id),
                            };
                            move |database| {
                                Box::pin(async move {
                                    database
                                        .query_first::<FirstFolderRow, _>(
                                            "SELECT folder_id, parent_folder_id, name, project_ref, created_at FROM $folder WHERE workspace_id = $workspace LIMIT 1;",
                                            bindings,
                                        )
                                        .await
                                })
                            }
                        })
                        .await
                        .map_err(map_err)?
                }
            };
        }
        ancestry.reverse();
        if let Some(project_ref) = ancestry.iter().find_map(|folder| {
            folder
                .project_ref
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        }) {
            crumbs.push(LoomBreadcrumb {
                kind: "project".to_owned(),
                id: project_ref.to_owned(),
                label: project_ref.to_owned(),
            });
        }
        crumbs.extend(ancestry.into_iter().map(|folder| LoomBreadcrumb {
            kind: "folder".to_owned(),
            id: folder.folder_id,
            label: folder.name,
        }));
    }

    let block_label = block
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| format!("{} {}", block.content_type.as_str(), block.block_id));
    crumbs.push(LoomBreadcrumb {
        kind: "block".to_owned(),
        id: block.block_id.clone(),
        label: block_label,
    });
    let bridge = storage
        .with_data_operation({
            let bindings = WorkspaceBlockBinding {
                workspace: thing(WORKSPACES, workspace_id),
                block: thing(BLOCKS, block_id),
            };
            move |database| {
                Box::pin(async move {
                    database
                        .query_first::<BridgeEntityRow, _>(
                            "SELECT entity_id FROM loom_block_knowledge_bridge WHERE workspace_id = $workspace AND block_id = $block LIMIT 1;",
                            bindings,
                        )
                        .await
                })
            }
        })
        .await
        .map_err(map_err)?;
    if let Some(bridge) = bridge {
        crumbs.push(LoomBreadcrumb {
            kind: "entity".to_owned(),
            id: record_key(bridge.entity_id, ENTITIES)?,
            label: "knowledge_entity".to_owned(),
        });
    }
    Ok(LoomBreadcrumbTrail {
        block_id: block.block_id,
        crumbs,
    })
}

