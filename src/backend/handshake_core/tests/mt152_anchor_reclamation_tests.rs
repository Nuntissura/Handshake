#![cfg(all(feature = "surreal-test-support", feature = "test-utils"))]
//! MT-152 I-152-1: reclamation of `knowledge_rich_document_title_anchors`
//! rows using the anchor row itself as the conflict key.
//!
//! Invariant under proof (doc comment on `TitleAnchor` in
//! `storage/surreal/knowledge.rs`): an anchor row exists for a title iff at
//! least one live document in the workspace holds that normalized title -
//! eventually, with the transient window bounded by one conflicting
//! transaction. Every title-mutating transaction UPSERTs the anchor, so the
//! reclaiming transaction (atomic delete, rename away from the title) collides
//! at commit with a concurrent create of the same title: the loser's bounded
//! retry either recreates the anchor (create) or re-counts and keeps it
//! (reclaim). No live document is ever left without its anchor.
//!
//! Two `SurrealDatabase` wrappers over one engine, one keyed and one with
//! `LockMode::Disabled`, so the database transactions alone decide.

#[path = "knowledge_ingestion_support/mod.rs"]
mod embedded_knowledge_support;

use std::sync::Arc;

use embedded_knowledge_support::{open_embedded_store, EmbeddedKnowledgeStore};
use handshake_core::kernel::{KernelActor, KernelEventType, NewKernelEvent};
use handshake_core::storage::knowledge::{
    KnowledgeRichDocument, KnowledgeStore, NewKnowledgeRichDocument,
};
use handshake_core::storage::surreal::keyed_lock::{KeyedLockRegistry, LockMode};
use handshake_core::storage::surreal::{RowFilter, SurrealDatabase};
use serde_json::{json, Value};
use tokio::sync::Barrier;
use uuid::Uuid;

const ANCHOR_TABLE: &str = "knowledge_rich_document_title_anchors";
const RACE_ITERATIONS: usize = 6;

fn independent_wrappers(store_db: &SurrealDatabase) -> (SurrealDatabase, SurrealDatabase) {
    let keyed = store_db.clone();
    let disabled = SurrealDatabase::with_lock_registry(
        store_db.storage().clone(),
        KeyedLockRegistry::disabled(),
    );
    assert_eq!(keyed.lock_registry().mode(), LockMode::Keyed);
    assert_eq!(disabled.lock_registry().mode(), LockMode::Disabled);
    (keyed, disabled)
}

/// Mirror of the store's `normalize_rich_document_title`: trim, lowercase,
/// collapse whitespace.
fn title_key(title: &str) -> String {
    title
        .trim()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
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

async fn create_document(
    db: &SurrealDatabase,
    workspace_id: &str,
    title: &str,
) -> KnowledgeRichDocument {
    KnowledgeStore::create_knowledge_rich_document(
        db,
        NewKnowledgeRichDocument {
            workspace_id: workspace_id.to_owned(),
            title: title.to_owned(),
            schema_version: "hsk_richdoc_v1".to_owned(),
            content_json: rich_content(&format!("body {}", Uuid::now_v7())),
            ..Default::default()
        },
    )
    .await
    .expect("create rich document")
}

fn delete_event(document: &KnowledgeRichDocument) -> NewKernelEvent {
    NewKernelEvent::builder(
        "KTR-mt152-anchor-reclamation",
        "session-mt152-anchor-reclamation",
        KernelEventType::KnowledgeRichDocumentDeleted,
        KernelActor::System("mt152-anchor-reclamation-tests".to_owned()),
    )
    .aggregate("knowledge_rich_document", &document.rich_document_id)
    .idempotency_key(format!("mt152-delete:{}", Uuid::now_v7()))
    .source_component("mt152_anchor_reclamation_tests")
    .payload(json!({
        "event": "deleted",
        "workspace_id": document.workspace_id,
        "doc_version": document.doc_version,
        "title": document.title,
    }))
    .build()
    .expect("valid delete receipt event")
}

async fn delete_document(db: &SurrealDatabase, document: &KnowledgeRichDocument) {
    db.delete_knowledge_rich_document_atomic(document, delete_event(document))
        .await
        .expect("atomic delete");
}

fn scalar_string(field: &str, value: &Value) -> String {
    let unwrapped = value
        .get("value")
        .or_else(|| value.get("string"))
        .or_else(|| value.get("String"))
        .unwrap_or(value);
    unwrapped
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| panic!("projected {field} is not a string: {value:?}"))
}

/// Anchor rows for the whole store as (title_key, last_rich_document_id).
async fn anchor_rows(store: &EmbeddedKnowledgeStore) -> Vec<(String, String)> {
    let inspector = store.storage.test_inspector();
    let table = inspector
        .table_selector(ANCHOR_TABLE)
        .await
        .expect("title anchor table selector");
    inspector
        .project(
            &table,
            &[
                table.field("title_key").expect("title_key field"),
                table
                    .field("last_rich_document_id")
                    .expect("last_rich_document_id field"),
            ],
            RowFilter::All,
        )
        .await
        .expect("project title anchors")
        .into_iter()
        .map(|row| {
            (
                scalar_string("title_key", &row.values["title_key"]),
                scalar_string(
                    "last_rich_document_id",
                    &row.values["last_rich_document_id"],
                ),
            )
        })
        .collect()
}

async fn anchors_for(store: &EmbeddedKnowledgeStore, key: &str) -> usize {
    anchor_rows(store)
        .await
        .iter()
        .filter(|(title_key, _)| title_key == key)
        .count()
}

async fn live_holders(
    db: &SurrealDatabase,
    workspace_id: &str,
    key: &str,
) -> Vec<KnowledgeRichDocument> {
    KnowledgeStore::list_knowledge_rich_documents(db, workspace_id, None, None)
        .await
        .expect("list rich documents")
        .into_iter()
        .filter(|document| title_key(&document.title) == key)
        .collect()
}

/// The invariant at a quiescent point: anchor count is 1 when holders exist,
/// 0 otherwise, and never 0 while a live document holds the title.
async fn assert_anchor_invariant(
    store: &EmbeddedKnowledgeStore,
    db: &SurrealDatabase,
    workspace_id: &str,
    title: &str,
    expected_live: usize,
    label: &str,
) -> Vec<KnowledgeRichDocument> {
    let key = title_key(title);
    let holders = live_holders(db, workspace_id, &key).await;
    let anchors = anchors_for(store, &key).await;
    assert_eq!(
        holders.len(),
        expected_live,
        "{label}: live holders of {title:?}"
    );
    assert_eq!(
        anchors,
        usize::from(expected_live > 0),
        "{label}: anchor rows for {title:?} with {} live holder(s)",
        holders.len()
    );
    holders
}

/// (a) delete of the last holder racing a create of the same title, both
/// spawn orders: exactly one anchor and one live document afterwards, both
/// calls succeed (the loser's retry converges instead of surfacing a conflict).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn delete_of_last_holder_racing_create_keeps_exactly_one_anchor() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (keyed, disabled) = independent_wrappers(&store.db);

    for iteration in 0..RACE_ITERATIONS {
        let title = format!("Reclaim Race {iteration}");
        let victim = create_document(&keyed, &workspace_id, &title).await;
        assert_anchor_invariant(&store, &keyed, &workspace_id, &title, 1, "before race").await;

        let barrier = Arc::new(Barrier::new(2));
        let delete_db = if iteration % 2 == 0 { disabled.clone() } else { keyed.clone() };
        let create_db = if iteration % 2 == 0 { keyed.clone() } else { disabled.clone() };
        let delete_task = {
            let barrier = Arc::clone(&barrier);
            let victim = victim.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                delete_db
                    .delete_knowledge_rich_document_atomic(&victim, delete_event(&victim))
                    .await
            })
        };
        let create_task = {
            let barrier = Arc::clone(&barrier);
            let workspace_id = workspace_id.clone();
            let title = title.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                KnowledgeStore::create_knowledge_rich_document(
                    &create_db,
                    NewKnowledgeRichDocument {
                        workspace_id,
                        title,
                        schema_version: "hsk_richdoc_v1".to_owned(),
                        content_json: rich_content(&format!("racer {iteration}")),
                        ..Default::default()
                    },
                )
                .await
            })
        };
        // Spawn order alternates with the wrapper assignment; await order too.
        let (delete_outcome, create_outcome) = if iteration % 2 == 0 {
            let d = delete_task.await.expect("delete task");
            let c = create_task.await.expect("create task");
            (d, c)
        } else {
            let c = create_task.await.expect("create task");
            let d = delete_task.await.expect("delete task");
            (d, c)
        };
        let deleted = delete_outcome.expect("delete must converge through its bounded retry");
        let created = create_outcome.expect("create must converge through its bounded retry");
        assert!(deleted.loom_block_deleted);

        let holders =
            assert_anchor_invariant(&store, &keyed, &workspace_id, &title, 1, "after race").await;
        assert_eq!(holders[0].rich_document_id, created.rich_document_id);

        // (c) folded into the loop tail: deleting the last holder leaves zero anchors.
        delete_document(&disabled, &created).await;
        assert_anchor_invariant(&store, &keyed, &workspace_id, &title, 0, "after cleanup").await;
    }
    store
        .close_and_remove()
        .await
        .expect("close and remove the reclamation store");
}

/// (b) rename away from a title racing a create of that title, both orders:
/// the old title ends with exactly one anchor (the new document's) and one
/// live holder, the new title with one anchor and one holder.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rename_away_racing_create_of_the_old_title_keeps_exactly_one_anchor() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (keyed, disabled) = independent_wrappers(&store.db);

    for iteration in 0..RACE_ITERATIONS {
        let old_title = format!("Rename Race {iteration}");
        let new_title = format!("Renamed Away {iteration}");
        let mover = create_document(&keyed, &workspace_id, &old_title).await;
        assert_anchor_invariant(&store, &keyed, &workspace_id, &old_title, 1, "before race")
            .await;

        let barrier = Arc::new(Barrier::new(2));
        let rename_db = if iteration % 2 == 0 { disabled.clone() } else { keyed.clone() };
        let create_db = if iteration % 2 == 0 { keyed.clone() } else { disabled.clone() };
        let rename_task = {
            let barrier = Arc::clone(&barrier);
            let mover_id = mover.rich_document_id.clone();
            let new_title = new_title.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                KnowledgeStore::rename_knowledge_rich_document(
                    &rename_db,
                    &mover_id,
                    &new_title,
                    None,
                )
                .await
            })
        };
        let create_task = {
            let barrier = Arc::clone(&barrier);
            let workspace_id = workspace_id.clone();
            let old_title = old_title.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                KnowledgeStore::create_knowledge_rich_document(
                    &create_db,
                    NewKnowledgeRichDocument {
                        workspace_id,
                        title: old_title,
                        schema_version: "hsk_richdoc_v1".to_owned(),
                        content_json: rich_content(&format!("rename racer {iteration}")),
                        ..Default::default()
                    },
                )
                .await
            })
        };
        let (rename_outcome, create_outcome) = if iteration % 2 == 0 {
            let r = rename_task.await.expect("rename task");
            let c = create_task.await.expect("create task");
            (r, c)
        } else {
            let c = create_task.await.expect("create task");
            let r = rename_task.await.expect("rename task");
            (r, c)
        };
        let renamed = rename_outcome.expect("rename must converge through its bounded retry");
        let created = create_outcome.expect("create must converge through its bounded retry");
        assert_eq!(renamed.title, new_title);

        let old_holders =
            assert_anchor_invariant(&store, &keyed, &workspace_id, &old_title, 1, "old title")
                .await;
        assert_eq!(old_holders[0].rich_document_id, created.rich_document_id);
        let new_holders =
            assert_anchor_invariant(&store, &keyed, &workspace_id, &new_title, 1, "new title")
                .await;
        assert_eq!(new_holders[0].rich_document_id, mover.rich_document_id);

        delete_document(&keyed, &created).await;
        delete_document(&disabled, &renamed).await;
        assert_anchor_invariant(&store, &keyed, &workspace_id, &old_title, 0, "old cleanup")
            .await;
        assert_anchor_invariant(&store, &keyed, &workspace_id, &new_title, 0, "new cleanup")
            .await;
    }
    store
        .close_and_remove()
        .await
        .expect("close and remove the reclamation store");
}

/// (c) a plain delete of the last holder leaves zero anchors for the title,
/// and a plain rename away from a sole-holder title reclaims the old anchor
/// while the new title gains one.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn plain_delete_and_rename_of_the_last_holder_reclaim_the_anchor() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (keyed, disabled) = independent_wrappers(&store.db);

    let title = "Sole Holder";
    let document = create_document(&disabled, &workspace_id, title).await;
    assert_anchor_invariant(&store, &keyed, &workspace_id, title, 1, "after create").await;
    delete_document(&disabled, &document).await;
    assert_anchor_invariant(&store, &keyed, &workspace_id, title, 0, "after delete").await;

    let mover = create_document(&keyed, &workspace_id, "Before Rename").await;
    assert_anchor_invariant(&store, &keyed, &workspace_id, "Before Rename", 1, "created").await;
    let renamed = KnowledgeStore::rename_knowledge_rich_document(
        &disabled,
        &mover.rich_document_id,
        "After Rename",
        None,
    )
    .await
    .expect("rename sole holder");
    assert_eq!(renamed.title, "After Rename");
    assert_anchor_invariant(&store, &keyed, &workspace_id, "Before Rename", 0, "old title")
        .await;
    assert_anchor_invariant(&store, &keyed, &workspace_id, "After Rename", 1, "new title").await;
    // A case-only rename keeps the single anchor (same normalized key).
    let recased = KnowledgeStore::rename_knowledge_rich_document(
        &keyed,
        &mover.rich_document_id,
        "after rename",
        None,
    )
    .await
    .expect("case-only rename");
    assert_eq!(recased.title, "after rename");
    assert_anchor_invariant(&store, &keyed, &workspace_id, "After Rename", 1, "recased").await;
    delete_document(&keyed, &recased).await;
    assert_anchor_invariant(&store, &keyed, &workspace_id, "After Rename", 0, "cleanup").await;
    store
        .close_and_remove()
        .await
        .expect("close and remove the reclamation store");
}

/// (d) two live documents holding one normalized title (legal on the plain
/// create path; the variants differ in case and whitespace so the SurrealQL
/// normalization inside the reclaim is proven equal to the Rust one):
/// deleting one keeps the anchor, deleting the other reclaims it.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleting_one_of_two_holders_keeps_the_anchor_until_the_last_goes() {
    let Some(store) = open_embedded_store().await else {
        return;
    };
    let workspace_id = store.create_workspace().await;
    let (keyed, disabled) = independent_wrappers(&store.db);

    let first = create_document(&keyed, &workspace_id, "Shared   Title").await;
    let second = create_document(&disabled, &workspace_id, "shared TITLE").await;
    assert_eq!(title_key(&first.title), title_key(&second.title));
    let holders =
        assert_anchor_invariant(&store, &keyed, &workspace_id, "shared title", 2, "two holders")
            .await;
    assert_eq!(holders.len(), 2);

    delete_document(&disabled, &first).await;
    let holders =
        assert_anchor_invariant(&store, &keyed, &workspace_id, "shared title", 1, "one holder")
            .await;
    assert_eq!(holders[0].rich_document_id, second.rich_document_id);

    delete_document(&keyed, &second).await;
    assert_anchor_invariant(&store, &keyed, &workspace_id, "shared title", 0, "no holder").await;
    store
        .close_and_remove()
        .await
        .expect("close and remove the reclamation store");
}
