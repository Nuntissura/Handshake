//! MT-136 proof for the cross-family RichDocument delete transaction.
//!
//! The test uses the production adapter over a fresh embedded RocksDB store,
//! injects a late Loom delete failure to prove transaction rollback, then
//! proves successful cleanup, repeat-delete idempotence, and restart durability.

use serde_json::{json, Value};
use surrealdb::types::{Datetime, RecordId, RecordIdKey, SurrealValue, Value as SurrealValueData};

use super::{
    mt136_proof_harness::{embedded_proof_backend, EmbeddedProofBackend},
    SurrealDatabase, SurrealStorage,
};
use crate::{
    kernel::{KernelActor, KernelEventType, NewKernelEvent},
    storage::{
        knowledge::{
            KnowledgePermissionScope, KnowledgeRedactionState, KnowledgeSourceKind, KnowledgeStore,
            NewKnowledgeRichDocument, NewKnowledgeSource, UpsertKnowledgeDocumentBacklink,
            UpsertKnowledgeRichDocumentDraft,
        },
        Database, LoomBlockContentType, LoomBlockDerived, NewLoomBlock, NewLoomCanvasPlacement,
        NewWorkspace, StorageError, StorageResult, WriteContext, LOOM_CANVAS_BOARD_SCHEMA_ID,
    },
};

fn ctx() -> WriteContext {
    WriteContext::human(Some("mt136-rich-document-delete-proof".to_owned()))
}

fn rich_content(text: &str) -> Value {
    json!({
        "type": "doc",
        "content": [{
            "type": "paragraph",
            "content": [{"type": "text", "text": text}]
        }]
    })
}

fn delete_event(document_id: &str, workspace_id: &str, title: &str) -> NewKernelEvent {
    NewKernelEvent::builder(
        "KTR-mt136-rich-delete",
        "session-mt136-rich-delete",
        KernelEventType::KnowledgeRichDocumentDeleted,
        KernelActor::System("mt136-rich-document-delete-proof".to_owned()),
    )
    .aggregate("knowledge_rich_document", document_id)
    .idempotency_key("mt136-rich-document-delete")
    .source_component("mt136_rich_document_delete_proof")
    .payload(json!({
        "event": "deleted",
        "workspace_id": workspace_id,
        "doc_version": 1,
        "title": title,
    }))
    .build()
    .expect("valid rich-document delete proof event")
}

async fn create_canvas_block(database: &dyn Database, workspace_id: &str) -> StorageResult<String> {
    let block = database
        .create_loom_block(
            &ctx(),
            NewLoomBlock {
                block_id: None,
                workspace_id: workspace_id.to_owned(),
                content_type: LoomBlockContentType::Canvas,
                document_id: None,
                asset_id: None,
                title: Some("Delete proof canvas".to_owned()),
                original_filename: None,
                content_hash: None,
                pinned: false,
                journal_date: None,
                imported_at: None,
                derived: LoomBlockDerived::default(),
            },
        )
        .await?;
    database
        .bridge_loom_block_to_knowledge(&ctx(), workspace_id, &block.block_id)
        .await?;
    database
        .create_canvas_board(
            &ctx(),
            workspace_id,
            &block.block_id,
            json!({
                "schema_id": LOOM_CANVAS_BOARD_SCHEMA_ID,
                "pan_x": 0.0,
                "pan_y": 0.0,
                "zoom": 1.0,
            }),
        )
        .await?;
    Ok(block.block_id)
}

#[derive(SurrealValue)]
struct TombstoneRow {
    deleted_at: Option<Datetime>,
    deleted_receipt_event_id: Option<RecordId>,
}

#[derive(SurrealValue)]
struct SourceRow {
    stale: bool,
}

#[derive(SurrealValue)]
struct EventRow {
    event_id: String,
}

#[derive(Debug, PartialEq, Eq)]
struct DeleteState {
    deleted: bool,
    receipt_event_id: Option<String>,
    source_stale: bool,
    block_count: usize,
    search_count: usize,
    backlink_count: usize,
    draft_count: usize,
    placement_count: usize,
    receipt_count: usize,
}

fn record_key(record: RecordId) -> StorageResult<String> {
    match record.key {
        RecordIdKey::String(value) => Ok(value),
        _ => Err(StorageError::Serialization(
            "delete proof receipt link is not a string record key".to_owned(),
        )),
    }
}

async fn inspect_state(
    storage: &SurrealStorage,
    workspace_id: &str,
    document_id: &str,
    title: &str,
    source_id: &str,
    placement_id: &str,
) -> StorageResult<DeleteState> {
    let workspace = RecordId::new("workspaces", workspace_id.to_owned());
    let document = RecordId::new("knowledge_rich_documents", document_id.to_owned());
    let block = RecordId::new("loom_blocks", document_id.to_owned());
    let search = RecordId::new("loom_block_search_index", document_id.to_owned());
    let source = RecordId::new("knowledge_sources", source_id.to_owned());
    let placement = RecordId::new("loom_canvas_placements", placement_id.to_owned());
    let document_id = document_id.to_owned();
    let title = title.to_owned();
    let result: Result<
        (
            Vec<TombstoneRow>,
            Vec<SourceRow>,
            Vec<SurrealValueData>,
            Vec<SurrealValueData>,
            Vec<SurrealValueData>,
            Vec<SurrealValueData>,
            Vec<SurrealValueData>,
            Vec<EventRow>,
        ),
        super::SurrealStorageError,
    > = storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                let mut response = database
                    .client
                    .query(
                        "SELECT deleted_at, deleted_receipt_event_id FROM $document; \
                         SELECT stale FROM $source; \
                         SELECT id FROM $block; \
                         SELECT id FROM $search; \
                         SELECT id FROM knowledge_document_backlinks WHERE workspace_id = $workspace \
                           AND (source_document_id = $document OR target = $document_id OR target = $title); \
                         SELECT id FROM knowledge_rich_document_drafts WHERE rich_document_id = $document; \
                         SELECT id FROM $placement; \
                         SELECT event_id FROM kernel_event_ledger WHERE idempotency_key = 'mt136-rich-document-delete';",
                    )
                    .bind(("workspace", workspace))
                    .bind(("document", document))
                    .bind(("document_id", document_id))
                    .bind(("title", title))
                    .bind(("block", block))
                    .bind(("search", search))
                    .bind(("source", source))
                    .bind(("placement", placement))
                    .await?
                    .check()?;
                Ok((
                    response.take(0)?,
                    response.take(1)?,
                    response.take(2)?,
                    response.take(3)?,
                    response.take(4)?,
                    response.take(5)?,
                    response.take(6)?,
                    response.take(7)?,
                ))
            })
        })
        .await;
    let (tombstones, sources, blocks, search, backlinks, drafts, placements, receipts) =
        result.map_err(|error| StorageError::Database(error.to_string()))?;
    let tombstone = tombstones.into_iter().next().ok_or_else(|| {
        StorageError::Database("delete proof lost RichDocument authority row".to_owned())
    })?;
    Ok(DeleteState {
        deleted: tombstone.deleted_at.is_some(),
        receipt_event_id: tombstone
            .deleted_receipt_event_id
            .map(record_key)
            .transpose()?,
        source_stale: sources.into_iter().next().is_some_and(|row| row.stale),
        block_count: blocks.len(),
        search_count: search.len(),
        backlink_count: backlinks.len(),
        draft_count: drafts.len(),
        placement_count: placements.len(),
        receipt_count: receipts.len(),
    })
}

async fn execute_ddl(storage: &SurrealStorage, statement: &'static str) -> StorageResult<()> {
    storage
        .with_data_operation(move |database| {
            Box::pin(async move {
                database.client.query(statement).await?.check()?;
                Ok(())
            })
        })
        .await
        .map_err(|error| StorageError::Database(error.to_string()))
}

async fn reopen(backend: EmbeddedProofBackend) -> StorageResult<EmbeddedProofBackend> {
    backend.reopen().await
}

async fn atomic_rich_document_delete_rolls_back_cleans_and_survives_reopen() -> StorageResult<()> {
    let backend = embedded_proof_backend().await?;
    let database = SurrealDatabase::new(backend.storage.clone());
    let workspace = backend
        .database
        .create_workspace(
            &ctx(),
            NewWorkspace {
                name: "MT-136 rich-document delete proof".to_owned(),
            },
        )
        .await?;
    let target = KnowledgeStore::create_knowledge_rich_document(
        &database,
        NewKnowledgeRichDocument {
            workspace_id: workspace.id.clone(),
            title: "Delete target".to_owned(),
            schema_version: "hsk_richdoc_v1".to_owned(),
            content_json: rich_content("durable target"),
            ..Default::default()
        },
    )
    .await?;
    let referring = KnowledgeStore::create_knowledge_rich_document(
        &database,
        NewKnowledgeRichDocument {
            workspace_id: workspace.id.clone(),
            title: "Referring document".to_owned(),
            schema_version: "hsk_richdoc_v1".to_owned(),
            content_json: rich_content("referring document"),
            ..Default::default()
        },
    )
    .await?;
    let source = KnowledgeStore::upsert_knowledge_source(
        &database,
        NewKnowledgeSource {
            workspace_id: workspace.id.clone(),
            root_id: None,
            source_kind: KnowledgeSourceKind::RichDocument,
            relative_path: None,
            asset_id: None,
            loom_block_id: None,
            document_id: None,
            content_hash: target.content_sha256.clone(),
            size_bytes: None,
            provenance: json!({"rich_document_id": target.rich_document_id.clone()}),
            permission_scope: KnowledgePermissionScope::Workspace,
            redaction_state: KnowledgeRedactionState::None,
            source_modified_at: None,
        },
    )
    .await?;
    KnowledgeStore::replace_knowledge_document_backlinks(
        &database,
        &referring.rich_document_id,
        vec![UpsertKnowledgeDocumentBacklink {
            workspace_id: workspace.id.clone(),
            relationship_id: format!("KDLNK-{}", "a".repeat(64)),
            source_document_id: referring.rich_document_id.clone(),
            link_kind: "wikilink".to_owned(),
            target: target.rich_document_id.clone(),
            block_id: "body.0".to_owned(),
        }],
    )
    .await?;
    KnowledgeStore::upsert_knowledge_rich_document_draft(
        &database,
        UpsertKnowledgeRichDocumentDraft {
            rich_document_id: target.rich_document_id.clone(),
            base_doc_version: target.doc_version,
            base_content_sha256: target.content_sha256.clone(),
            content_json: rich_content("unsaved draft"),
            actor_kind: "system".to_owned(),
            actor_id: "mt136-rich-document-delete-proof".to_owned(),
            kernel_task_run_id: "KTR-mt136-rich-delete".to_owned(),
            session_run_id: "session-mt136-rich-delete".to_owned(),
        },
    )
    .await?;
    let canvas = create_canvas_block(backend.database.as_ref(), &workspace.id).await?;
    let placement = backend
        .database
        .place_block_on_canvas(
            &ctx(),
            NewLoomCanvasPlacement {
                canvas_block_id: canvas,
                workspace_id: workspace.id.clone(),
                placed_block_id: target.block_id.clone(),
                x: 0.0,
                y: 0.0,
                w: 320.0,
                h: 180.0,
                z_index: 0,
                group_id: None,
                is_text_card: false,
                stage_provenance_key: None,
            },
        )
        .await?;
    let event = delete_event(&target.rich_document_id, &workspace.id, &target.title);

    execute_ddl(
        &backend.storage,
        "DEFINE EVENT OVERWRITE mt136_rich_delete_rollback_probe ON TABLE loom_blocks \
         WHEN $event = 'DELETE' THEN { THROW 'MT136-RICH-DELETE-ROLLBACK-PROBE'; };",
    )
    .await?;
    let failed = database
        .delete_knowledge_rich_document_atomic(&target, event.clone())
        .await;
    assert!(matches!(failed, Err(StorageError::Database(_))));
    let rolled_back = inspect_state(
        &backend.storage,
        &workspace.id,
        &target.rich_document_id,
        &target.title,
        &source.source_id,
        &placement.placement_id,
    )
    .await?;
    assert_eq!(
        rolled_back,
        DeleteState {
            deleted: false,
            receipt_event_id: None,
            source_stale: false,
            block_count: 1,
            search_count: 1,
            backlink_count: 1,
            draft_count: 1,
            placement_count: 1,
            receipt_count: 0,
        }
    );
    execute_ddl(
        &backend.storage,
        "REMOVE EVENT mt136_rich_delete_rollback_probe ON TABLE loom_blocks;",
    )
    .await?;

    let outcome = database
        .delete_knowledge_rich_document_atomic(&target, event.clone())
        .await?;
    assert!(outcome.source_marked_stale);
    assert_eq!(outcome.backlinks_deleted, 1);
    assert!(outcome.loom_block_deleted);
    let replayed = database
        .delete_knowledge_rich_document_atomic(&target, event.clone())
        .await?;
    assert_eq!(replayed, outcome);
    let mut divergent_event = event.clone();
    divergent_event.source_component = "mt136_rich_document_delete_proof_divergent".to_owned();
    assert!(matches!(
        database
            .delete_knowledge_rich_document_atomic(&target, divergent_event)
            .await,
        Err(StorageError::Conflict(_))
    ));
    let committed = inspect_state(
        &backend.storage,
        &workspace.id,
        &target.rich_document_id,
        &target.title,
        &source.source_id,
        &placement.placement_id,
    )
    .await?;
    assert!(committed.deleted);
    assert_eq!(
        committed.receipt_event_id,
        Some(outcome.receipt_event_id.clone())
    );
    assert!(committed.source_stale);
    assert_eq!(committed.block_count, 0);
    assert_eq!(committed.search_count, 0);
    assert_eq!(committed.backlink_count, 0);
    assert_eq!(committed.draft_count, 0);
    assert_eq!(committed.placement_count, 0);
    assert_eq!(committed.receipt_count, 1);

    drop(database);
    let backend = reopen(backend).await?;
    let reopened = inspect_state(
        &backend.storage,
        &workspace.id,
        &target.rich_document_id,
        &target.title,
        &source.source_id,
        &placement.placement_id,
    )
    .await?;
    assert_eq!(reopened, committed);
    let reopened_database = SurrealDatabase::new(backend.storage.clone());
    let replayed_after_reopen = reopened_database
        .delete_knowledge_rich_document_atomic(&target, event)
        .await?;
    assert_eq!(replayed_after_reopen, outcome);
    backend.close_and_remove().await?;
    Ok(())
}

pub(super) async fn run_all() -> StorageResult<()> {
    atomic_rich_document_delete_rolls_back_cleans_and_survives_reopen().await
}

#[cfg(test)]
#[tokio::test]
async fn mt136_rich_document_delete_proof() -> StorageResult<()> {
    run_all().await
}
