use async_trait::async_trait;
use chrono::Utc;
use handshake_core::storage::surreal::{SurrealStorage, SurrealStorageConfig};
use handshake_core::storage::{
    GuardError, MutationMetadata, NewDocument, NewWorkspace, StorageError, StorageGuard,
    WriteContext,
};
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

#[tokio::test]
async fn document_crud_enforces_parent_and_workspace_cascade() {
    let (_temp, storage) =
        open_store_with_guard(Arc::new(handshake_core::storage::DefaultStorageGuard)).await;
    let context = WriteContext::system(Some("surreal-document-test".to_owned()));
    let first_workspace = storage
        .create_workspace(
            &context,
            NewWorkspace {
                name: "First".to_owned(),
            },
        )
        .await
        .expect("create first workspace");
    let second_workspace = storage
        .create_workspace(
            &context,
            NewWorkspace {
                name: "Second".to_owned(),
            },
        )
        .await
        .expect("create second workspace");

    let first = storage
        .create_document(
            &context,
            NewDocument {
                workspace_id: first_workspace.id.clone(),
                title: "First document".to_owned(),
            },
        )
        .await
        .expect("create first document");
    let first_followup = storage
        .create_document(
            &context,
            NewDocument {
                workspace_id: first_workspace.id.clone(),
                title: "First workspace follow-up".to_owned(),
            },
        )
        .await
        .expect("create second document in first workspace");
    let second = storage
        .create_document(
            &context,
            NewDocument {
                workspace_id: second_workspace.id.clone(),
                title: "Second document".to_owned(),
            },
        )
        .await
        .expect("create second document");

    assert_eq!(
        Uuid::parse_str(&first.id)
            .expect("document id is a UUID")
            .get_version_num(),
        7
    );
    assert_eq!(first.workspace_id, first_workspace.id);
    assert_eq!(first.title, "First document");
    assert!(first.created_at <= first.updated_at);
    assert_eq!(
        storage
            .get_document(&first.id)
            .await
            .expect("get first document")
            .id,
        first.id
    );
    let first_documents = storage
        .list_documents(&first_workspace.id)
        .await
        .expect("list first workspace documents");
    assert_eq!(first_documents.len(), 2);
    assert!(first_documents
        .windows(2)
        .all(|pair| pair[0].created_at <= pair[1].created_at));
    assert!(first_documents
        .iter()
        .any(|document| document.id == first.id));
    assert!(first_documents
        .iter()
        .any(|document| document.id == first_followup.id));
    assert!(!first_documents
        .iter()
        .any(|document| document.id == second.id));
    let second_documents = storage
        .list_documents(&second_workspace.id)
        .await
        .expect("list second workspace documents");
    assert_eq!(second_documents.len(), 1);
    assert_eq!(second_documents[0].id, second.id);

    let first_before_failed_parent = first_documents
        .iter()
        .map(|document| (document.id.clone(), document.title.clone()))
        .collect::<Vec<_>>();
    let second_before_failed_parent = second_documents
        .iter()
        .map(|document| (document.id.clone(), document.title.clone()))
        .collect::<Vec<_>>();

    let missing_parent = Uuid::now_v7().to_string();
    assert!(matches!(
        storage
            .create_document(
                &context,
                NewDocument {
                    workspace_id: missing_parent,
                    title: "Orphan".to_owned(),
                },
            )
            .await,
        Err(StorageError::Database(_))
    ));
    let first_after_failed_parent = storage
        .list_documents(&first_workspace.id)
        .await
        .expect("list first workspace after rejected orphan")
        .into_iter()
        .map(|document| (document.id, document.title))
        .collect::<Vec<_>>();
    let second_after_failed_parent = storage
        .list_documents(&second_workspace.id)
        .await
        .expect("list second workspace after rejected orphan")
        .into_iter()
        .map(|document| (document.id, document.title))
        .collect::<Vec<_>>();
    assert_eq!(first_after_failed_parent, first_before_failed_parent);
    assert_eq!(second_after_failed_parent, second_before_failed_parent);
    assert!(!first_after_failed_parent
        .iter()
        .chain(second_after_failed_parent.iter())
        .any(|(_, title)| title == "Orphan"));

    storage
        .delete_workspace(&context, &first_workspace.id)
        .await
        .expect("delete parent workspace");
    assert!(matches!(
        storage.get_document(&first.id).await,
        Err(StorageError::NotFound("document"))
    ));
    assert!(matches!(
        storage.get_document(&first_followup.id).await,
        Err(StorageError::NotFound("document"))
    ));
    assert_eq!(
        storage
            .get_document(&second.id)
            .await
            .expect("unrelated document remains")
            .workspace_id,
        second_workspace.id
    );

    storage
        .delete_document(&context, &second.id)
        .await
        .expect("delete second document");
    assert!(matches!(
        storage.get_document(&second.id).await,
        Err(StorageError::NotFound("document"))
    ));
    assert!(matches!(
        storage.delete_document(&context, &second.id).await,
        Err(StorageError::NotFound("document"))
    ));

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn injected_guard_denial_preserves_document_state() {
    let guard = Arc::new(CountingDenyGuard::default());
    let (_temp, storage) = open_store_with_guard(guard.clone()).await;
    let allowed = WriteContext::system(Some("allow".to_owned()));
    let denied = WriteContext::system(Some("deny".to_owned()));
    let workspace = storage
        .create_workspace(
            &allowed,
            NewWorkspace {
                name: "Guarded".to_owned(),
            },
        )
        .await
        .expect("create guarded workspace");

    assert!(matches!(
        storage
            .create_document(
                &denied,
                NewDocument {
                    workspace_id: workspace.id.clone(),
                    title: "Rejected".to_owned(),
                },
            )
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(storage
        .list_documents(&workspace.id)
        .await
        .expect("list after denied create")
        .is_empty());

    let retained = storage
        .create_document(
            &allowed,
            NewDocument {
                workspace_id: workspace.id.clone(),
                title: "Retained".to_owned(),
            },
        )
        .await
        .expect("create retained document");
    assert!(matches!(
        storage.delete_document(&denied, &retained.id).await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert_eq!(
        storage
            .get_document(&retained.id)
            .await
            .expect("denied delete retains document")
            .title,
        "Retained"
    );
    assert_eq!(guard.calls.load(Ordering::SeqCst), 4);

    storage.shutdown().await.expect("close store");
}
