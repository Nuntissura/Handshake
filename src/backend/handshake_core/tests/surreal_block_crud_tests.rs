use async_trait::async_trait;
use chrono::Utc;
use handshake_core::storage::surreal::{SurrealStorage, SurrealStorageConfig};
use handshake_core::storage::{
    BlockUpdate, GuardError, MutationMetadata, NewBlock, NewDocument, NewWorkspace, StorageError,
    StorageGuard, WriteContext,
};
use serde_json::json;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use uuid::Uuid;

#[derive(Default)]
struct CountingDenyGuard {
    calls: AtomicUsize,
}

#[async_trait]
impl StorageGuard for CountingDenyGuard {
    async fn validate_write(
        &self,
        ctx: &WriteContext,
        resource_id: &str,
    ) -> Result<MutationMetadata, GuardError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if ctx.actor_id.as_deref() == Some("deny") {
            return Err(GuardError::SilentEdit);
        }
        Ok(MutationMetadata {
            actor_kind: ctx.actor_kind,
            actor_id: ctx.actor_id.clone(),
            job_id: ctx.job_id,
            workflow_id: ctx.workflow_id,
            edit_event_id: Uuid::now_v7(),
            resource_id: resource_id.to_owned(),
            timestamp: Utc::now(),
        })
    }
}

async fn open_store_with_guard(
    guard: Arc<dyn StorageGuard>,
) -> (tempfile::TempDir, SurrealStorage) {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let config = SurrealStorageConfig::for_data_dir(temp.path()).expect("configure store");
    let storage = SurrealStorage::open_with_guard(config, guard)
        .await
        .expect("open store");
    handshake_core::storage::surreal::bootstrap_schema(&storage)
        .await
        .expect("bootstrap schema");
    (temp, storage)
}

async fn create_document(storage: &SurrealStorage, ctx: &WriteContext) -> (String, String) {
    let workspace = storage
        .create_workspace(
            ctx,
            NewWorkspace {
                name: "Block workspace".to_owned(),
            },
        )
        .await
        .expect("create workspace");
    let document = storage
        .create_document(
            ctx,
            NewDocument {
                workspace_id: workspace.id.clone(),
                title: "Block document".to_owned(),
            },
        )
        .await
        .expect("create document");
    (workspace.id, document.id)
}

fn new_block(document_id: &str, id: Option<String>, sequence: i64, raw: &str) -> NewBlock {
    NewBlock {
        id,
        document_id: document_id.to_owned(),
        kind: "paragraph".to_owned(),
        sequence,
        raw_content: raw.to_owned(),
        display_content: None,
        derived_content: None,
        sensitivity: None,
        exportable: None,
    }
}

#[tokio::test]
async fn block_crud_preserves_trait_defaults_order_and_parent_cascade() {
    let (_temp, storage) =
        open_store_with_guard(Arc::new(handshake_core::storage::DefaultStorageGuard)).await;
    let ctx = WriteContext::system(Some("surreal-block-test".to_owned()));
    let (workspace_id, document_id) = create_document(&storage, &ctx).await;
    let unrelated_document = storage
        .create_document(
            &ctx,
            NewDocument {
                workspace_id,
                title: "Unrelated document".to_owned(),
            },
        )
        .await
        .expect("create unrelated document");
    let unrelated_block = storage
        .create_block(
            &ctx,
            new_block(&unrelated_document.id, None, 1, "Unrelated"),
        )
        .await
        .expect("create unrelated block");

    let later_id = Uuid::now_v7().to_string();
    let first_id = Uuid::now_v7().to_string();
    let later = storage
        .create_block(
            &ctx,
            NewBlock {
                id: Some(later_id.clone()),
                document_id: document_id.clone(),
                kind: "heading".to_owned(),
                sequence: 2,
                raw_content: "Later".to_owned(),
                display_content: Some("Later view".to_owned()),
                derived_content: Some(json!({"level": 2})),
                sensitivity: Some("low".to_owned()),
                exportable: Some(false),
            },
        )
        .await
        .expect("create later block");
    let first = storage
        .create_block(
            &ctx,
            new_block(&document_id, Some(first_id.clone()), 1, "First"),
        )
        .await
        .expect("create first block");

    assert_eq!(first.id, first_id);
    assert_eq!(first.display_content, "First");
    assert_eq!(first.derived_content, json!({}));
    assert!(first.created_at <= first.updated_at);
    assert_eq!(later.sensitivity.as_deref(), Some("low"));
    assert_eq!(later.exportable, Some(false));
    assert_eq!(
        storage
            .get_blocks(&document_id)
            .await
            .expect("list blocks")
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        vec![first_id.clone(), later_id.clone()]
    );

    storage
        .update_block(
            &ctx,
            &first.id,
            BlockUpdate {
                kind: Some("quote".to_owned()),
                sequence: Some(3),
                raw_content: Some("Updated raw".to_owned()),
                display_content: Some("Updated view".to_owned()),
                derived_content: Some(json!({"citation": "local"})),
            },
        )
        .await
        .expect("update first block");
    let updated = storage
        .get_block(&first.id)
        .await
        .expect("get updated block");
    assert_eq!(updated.kind, "quote");
    assert_eq!(updated.sequence, 3);
    assert_eq!(updated.raw_content, "Updated raw");
    assert_eq!(updated.display_content, "Updated view");
    assert_eq!(updated.derived_content, json!({"citation": "local"}));
    assert!(updated.updated_at >= updated.created_at);
    assert!(matches!(
        storage
            .update_block(
                &ctx,
                &updated.id,
                BlockUpdate {
                    kind: None,
                    sequence: None,
                    raw_content: None,
                    display_content: None,
                    derived_content: None,
                },
            )
            .await,
        Err(StorageError::Validation("no block fields provided"))
    ));

    let before_orphan = storage
        .get_blocks(&document_id)
        .await
        .expect("list before orphan rejection");
    assert!(matches!(
        storage
            .create_block(
                &ctx,
                new_block(&Uuid::now_v7().to_string(), None, 9, "Orphan"),
            )
            .await,
        Err(StorageError::Database(_))
    ));
    assert_eq!(
        storage
            .get_blocks(&document_id)
            .await
            .expect("list after orphan rejection")
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        before_orphan
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>()
    );

    storage
        .delete_document(&ctx, &document_id)
        .await
        .expect("delete parent document");
    assert!(matches!(
        storage.get_block(&later.id).await,
        Err(StorageError::NotFound("block"))
    ));
    assert!(matches!(
        storage.get_block(&first.id).await,
        Err(StorageError::NotFound("block"))
    ));
    assert_eq!(
        storage
            .get_block(&unrelated_block.id)
            .await
            .expect("unrelated child remains")
            .document_id,
        unrelated_document.id
    );

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn replace_blocks_is_atomic_and_preserves_caller_return_order() {
    let (_temp, storage) =
        open_store_with_guard(Arc::new(handshake_core::storage::DefaultStorageGuard)).await;
    let ctx = WriteContext::system(Some("surreal-block-replace-test".to_owned()));
    let (_workspace_id, document_id) = create_document(&storage, &ctx).await;
    let retained = storage
        .create_block(&ctx, new_block(&document_id, None, 1, "Retained"))
        .await
        .expect("create retained block");
    let duplicate_id = Uuid::now_v7().to_string();

    assert!(matches!(
        storage
            .replace_blocks(
                &ctx,
                &document_id,
                vec![
                    new_block(&document_id, Some(duplicate_id.clone()), 1, "Duplicate A"),
                    new_block(&document_id, Some(duplicate_id), 2, "Duplicate B"),
                ],
            )
            .await,
        Err(StorageError::Database(_))
    ));
    let after_failed_replace = storage
        .get_blocks(&document_id)
        .await
        .expect("list after failed replacement");
    assert_eq!(after_failed_replace.len(), 1);
    assert_eq!(after_failed_replace[0].id, retained.id);

    let second_id = Uuid::now_v7().to_string();
    let first_id = Uuid::now_v7().to_string();
    let replaced = storage
        .replace_blocks(
            &ctx,
            &document_id,
            vec![
                new_block(&document_id, Some(second_id.clone()), 2, "Second"),
                NewBlock {
                    derived_content: Some(json!({"order": 1})),
                    ..new_block(&document_id, Some(first_id.clone()), 1, "First")
                },
            ],
        )
        .await
        .expect("replace blocks");
    assert_eq!(
        replaced
            .iter()
            .map(|block| block.id.as_str())
            .collect::<Vec<_>>(),
        vec![second_id.as_str(), first_id.as_str()]
    );
    assert_eq!(replaced[1].display_content, "First");
    assert_eq!(replaced[1].derived_content, json!({"order": 1}));
    assert_eq!(
        storage
            .get_blocks(&document_id)
            .await
            .expect("list replacements in sequence order")
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        vec![first_id.clone(), second_id.clone()]
    );
    assert!(matches!(
        storage.get_block(&retained.id).await,
        Err(StorageError::NotFound("block"))
    ));

    storage
        .delete_block(&ctx, &first_id)
        .await
        .expect("delete replacement block");
    assert_eq!(
        storage
            .get_blocks(&document_id)
            .await
            .expect("list after block delete")
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        vec![second_id]
    );
    assert!(matches!(
        storage.delete_block(&ctx, &first_id).await,
        Err(StorageError::NotFound("block"))
    ));

    let before_mismatch = storage
        .get_blocks(&document_id)
        .await
        .expect("list before mismatched replacement");
    assert!(matches!(
        storage
            .replace_blocks(
                &ctx,
                &document_id,
                vec![new_block(
                    &Uuid::now_v7().to_string(),
                    None,
                    1,
                    "Wrong parent",
                )],
            )
            .await,
        Err(StorageError::Validation(
            "block document does not match replacement document"
        ))
    ));
    assert_eq!(
        storage
            .get_blocks(&document_id)
            .await
            .expect("mismatched replacement preserves state")
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>(),
        before_mismatch
            .into_iter()
            .map(|block| block.id)
            .collect::<Vec<_>>()
    );

    let missing_document = Uuid::now_v7().to_string();
    assert!(matches!(
        storage
            .replace_blocks(&ctx, &missing_document, Vec::new())
            .await,
        Err(StorageError::NotFound("document"))
    ));

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn concurrent_disjoint_updates_merge_without_lost_fields() {
    let (_temp, storage) =
        open_store_with_guard(Arc::new(handshake_core::storage::DefaultStorageGuard)).await;
    let ctx = WriteContext::system(Some("surreal-block-concurrency-test".to_owned()));
    let (_workspace_id, document_id) = create_document(&storage, &ctx).await;
    let block = storage
        .create_block(
            &ctx,
            NewBlock {
                display_content: Some(String::new()),
                ..new_block(&document_id, None, 1, "Initial")
            },
        )
        .await
        .expect("create concurrently updated block");
    let barrier = Arc::new(tokio::sync::Barrier::new(3));

    let raw_storage = storage.clone();
    let raw_ctx = ctx.clone();
    let raw_block_id = block.id.clone();
    let raw_barrier = barrier.clone();
    let raw_update = async move {
        raw_barrier.wait().await;
        raw_storage
            .update_block(
                &raw_ctx,
                &raw_block_id,
                BlockUpdate {
                    kind: None,
                    sequence: None,
                    raw_content: Some("Concurrent raw".to_owned()),
                    display_content: None,
                    derived_content: None,
                },
            )
            .await
    };
    let display_storage = storage.clone();
    let display_ctx = ctx.clone();
    let display_block_id = block.id.clone();
    let display_barrier = barrier.clone();
    let display_update = async move {
        display_barrier.wait().await;
        display_storage
            .update_block(
                &display_ctx,
                &display_block_id,
                BlockUpdate {
                    kind: None,
                    sequence: Some(9),
                    raw_content: None,
                    display_content: Some("Explicit concurrent view".to_owned()),
                    derived_content: None,
                },
            )
            .await
    };

    let (_, raw_result, display_result) = tokio::join!(barrier.wait(), raw_update, display_update);
    raw_result.expect("raw update commits");
    display_result.expect("display update commits");
    let updated = storage
        .get_block(&block.id)
        .await
        .expect("read concurrently updated block");
    assert_eq!(updated.raw_content, "Concurrent raw");
    assert_eq!(updated.display_content, "Explicit concurrent view");
    assert_eq!(updated.sequence, 9);

    let fallback = storage
        .create_block(
            &ctx,
            NewBlock {
                display_content: Some(String::new()),
                ..new_block(&document_id, None, 2, "Fallback initial")
            },
        )
        .await
        .expect("create fallback block");
    storage
        .update_block(
            &ctx,
            &fallback.id,
            BlockUpdate {
                kind: None,
                sequence: None,
                raw_content: Some("Fallback raw".to_owned()),
                display_content: None,
                derived_content: None,
            },
        )
        .await
        .expect("update fallback block");
    let fallback = storage
        .get_block(&fallback.id)
        .await
        .expect("read fallback block");
    assert_eq!(fallback.raw_content, "Fallback raw");
    assert_eq!(fallback.display_content, "Fallback raw");

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn replace_racing_parent_delete_never_creates_orphans_or_touches_unrelated_children() {
    let (_temp, storage) =
        open_store_with_guard(Arc::new(handshake_core::storage::DefaultStorageGuard)).await;
    let ctx = WriteContext::system(Some("surreal-block-parent-race-test".to_owned()));
    let (workspace_id, document_id) = create_document(&storage, &ctx).await;
    let unrelated_document = storage
        .create_document(
            &ctx,
            NewDocument {
                workspace_id,
                title: "Unrelated race document".to_owned(),
            },
        )
        .await
        .expect("create unrelated race document");
    let unrelated_block = storage
        .create_block(
            &ctx,
            new_block(&unrelated_document.id, None, 1, "Unrelated race block"),
        )
        .await
        .expect("create unrelated race block");
    let barrier = Arc::new(tokio::sync::Barrier::new(3));

    let replace_storage = storage.clone();
    let replace_ctx = ctx.clone();
    let replace_document_id = document_id.clone();
    let replace_barrier = barrier.clone();
    let replacement = async move {
        replace_barrier.wait().await;
        replace_storage
            .replace_blocks(
                &replace_ctx,
                &replace_document_id,
                vec![new_block(
                    &replace_document_id,
                    None,
                    1,
                    "Racing replacement",
                )],
            )
            .await
    };
    let delete_storage = storage.clone();
    let delete_ctx = ctx.clone();
    let delete_document_id = document_id.clone();
    let delete_barrier = barrier.clone();
    let deletion = async move {
        delete_barrier.wait().await;
        delete_storage
            .delete_document(&delete_ctx, &delete_document_id)
            .await
    };

    let (_, replacement_result, deletion_result) =
        tokio::join!(barrier.wait(), replacement, deletion);
    deletion_result.expect("parent deletion commits");
    assert!(matches!(
        replacement_result,
        Ok(_) | Err(StorageError::NotFound("document"))
    ));
    assert!(matches!(
        storage.get_document(&document_id).await,
        Err(StorageError::NotFound("document"))
    ));
    assert!(storage
        .get_blocks(&document_id)
        .await
        .expect("deleted parent has no orphan blocks")
        .is_empty());
    assert_eq!(
        storage
            .get_block(&unrelated_block.id)
            .await
            .expect("unrelated child survives race")
            .document_id,
        unrelated_document.id
    );

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn injected_guard_denials_preserve_block_state() {
    let guard = Arc::new(CountingDenyGuard::default());
    let (_temp, storage) = open_store_with_guard(guard.clone()).await;
    let allowed = WriteContext::system(Some("allow".to_owned()));
    let denied = WriteContext::system(Some("deny".to_owned()));
    let (_workspace_id, document_id) = create_document(&storage, &allowed).await;
    let retained = storage
        .create_block(&allowed, new_block(&document_id, None, 1, "Retained"))
        .await
        .expect("create retained block");

    assert!(matches!(
        storage
            .create_block(&denied, new_block(&document_id, None, 2, "Rejected"))
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(matches!(
        storage
            .replace_blocks(
                &denied,
                &document_id,
                vec![new_block(
                    &Uuid::now_v7().to_string(),
                    None,
                    1,
                    "Mismatched before guard",
                )],
            )
            .await,
        Err(StorageError::Validation(
            "block document does not match replacement document"
        ))
    ));
    assert!(matches!(
        storage
            .update_block(
                &denied,
                &retained.id,
                BlockUpdate {
                    kind: Some("rejected".to_owned()),
                    sequence: None,
                    raw_content: None,
                    display_content: None,
                    derived_content: None,
                },
            )
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(matches!(
        storage.delete_block(&denied, &retained.id).await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(matches!(
        storage
            .replace_blocks(
                &denied,
                &document_id,
                vec![new_block(&document_id, None, 1, "Rejected replace")],
            )
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));

    let after_denials = storage
        .get_blocks(&document_id)
        .await
        .expect("list after denied mutations");
    assert_eq!(after_denials.len(), 1);
    assert_eq!(after_denials[0].id, retained.id);
    assert_eq!(after_denials[0].kind, "paragraph");
    assert_eq!(after_denials[0].raw_content, "Retained");
    assert_eq!(guard.calls.load(Ordering::SeqCst), 7);

    storage.shutdown().await.expect("close store");
}
