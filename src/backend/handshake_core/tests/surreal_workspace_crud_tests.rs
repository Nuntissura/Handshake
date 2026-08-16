use async_trait::async_trait;
use chrono::Utc;
use handshake_core::storage::surreal::{SurrealStorage, SurrealStorageConfig};
use handshake_core::storage::{
    GuardError, MutationMetadata, NewWorkspace, StorageError, StorageGuard, WriteContext,
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

async fn open_store() -> (tempfile::TempDir, SurrealStorage) {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let config = SurrealStorageConfig::for_data_dir(temp.path()).expect("configure store");
    let storage = SurrealStorage::open(config).await.expect("open store");
    handshake_core::storage::surreal::bootstrap_schema(&storage)
        .await
        .expect("bootstrap schema");
    (temp, storage)
}

#[tokio::test]
async fn workspace_crud_uses_schema_defaults_and_typed_missing_error() {
    let (_temp, storage) = open_store().await;
    let write_context = WriteContext::system(Some("surreal-workspace-test".to_owned()));

    let first = storage
        .create_workspace(
            &write_context,
            NewWorkspace {
                name: "First".to_owned(),
            },
        )
        .await
        .expect("create first workspace");
    let second = storage
        .create_workspace(
            &write_context,
            NewWorkspace {
                name: "Second".to_owned(),
            },
        )
        .await
        .expect("create second workspace");

    assert_eq!(first.name, "First");
    assert_eq!(
        Uuid::parse_str(&first.id)
            .expect("workspace id is a UUID")
            .get_version_num(),
        7
    );
    assert!(first.created_at <= first.updated_at);
    let selected = storage
        .get_workspace(&first.id)
        .await
        .expect("get first workspace")
        .expect("first workspace exists");
    assert_eq!(selected.id, first.id);
    assert_eq!(selected.name, first.name);
    assert_eq!(selected.created_at, first.created_at);
    assert_eq!(selected.updated_at, first.updated_at);
    let listed = storage.list_workspaces().await.expect("list workspaces");
    assert_eq!(listed.len(), 2);
    assert!(listed.windows(2).all(|pair| {
        (pair[0].created_at, pair[0].id.as_str()) <= (pair[1].created_at, pair[1].id.as_str())
    }));
    assert!(listed.iter().any(|workspace| workspace.id == first.id));
    assert!(listed.iter().any(|workspace| workspace.id == second.id));

    storage
        .delete_workspace(&write_context, &first.id)
        .await
        .expect("delete first workspace");
    assert!(storage
        .get_workspace(&first.id)
        .await
        .expect("get deleted workspace")
        .is_none());
    assert!(matches!(
        storage.delete_workspace(&write_context, &first.id).await,
        Err(StorageError::NotFound("workspace"))
    ));

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn injected_guard_rejects_create_and_delete_without_mutating_records() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let config = SurrealStorageConfig::for_data_dir(temp.path()).expect("configure store");
    let guard = Arc::new(CountingDenyGuard::default());
    let storage = SurrealStorage::open_with_guard(config, guard.clone())
        .await
        .expect("open store with custom guard");
    handshake_core::storage::surreal::bootstrap_schema(&storage)
        .await
        .expect("bootstrap schema");
    let cloned_storage = storage.clone();
    let denied = WriteContext::system(Some("deny".to_owned()));
    let allowed = WriteContext::system(Some("allow".to_owned()));

    assert!(matches!(
        cloned_storage
            .create_workspace(
                &denied,
                NewWorkspace {
                    name: "Rejected".to_owned(),
                },
            )
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(storage
        .list_workspaces()
        .await
        .expect("list after rejected create")
        .is_empty());

    let retained = storage
        .create_workspace(
            &allowed,
            NewWorkspace {
                name: "Retained".to_owned(),
            },
        )
        .await
        .expect("custom guard allows create");
    assert!(matches!(
        cloned_storage.delete_workspace(&denied, &retained.id).await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(storage
        .get_workspace(&retained.id)
        .await
        .expect("read retained target")
        .is_some());
    assert_eq!(guard.calls.load(Ordering::SeqCst), 3);

    storage.shutdown().await.expect("close store");
}

#[tokio::test]
async fn workspace_ai_write_requires_job_and_workflow_metadata() {
    let (_temp, storage) = open_store().await;
    let system_context = WriteContext::system(Some("workspace-test".to_owned()));
    let missing_job_context =
        WriteContext::ai(Some("agent".to_owned()), None, Some(Uuid::now_v7()));
    let missing_workflow_context =
        WriteContext::ai(Some("agent".to_owned()), Some(Uuid::now_v7()), None);
    let existing = storage
        .create_workspace(
            &system_context,
            NewWorkspace {
                name: "Existing".to_owned(),
            },
        )
        .await
        .expect("create workspace for guarded delete");
    let complete_ai_context = WriteContext::ai(
        Some("agent".to_owned()),
        Some(Uuid::now_v7()),
        Some(Uuid::now_v7()),
    );
    let ai_authored = storage
        .create_workspace(
            &complete_ai_context,
            NewWorkspace {
                name: "AI-authored".to_owned(),
            },
        )
        .await
        .expect("complete AI traceability metadata is accepted");

    assert!(matches!(
        storage
            .create_workspace(
                &missing_job_context,
                NewWorkspace {
                    name: "Blocked".to_owned(),
                },
            )
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    assert!(matches!(
        storage
            .delete_workspace(&missing_workflow_context, &existing.id)
            .await,
        Err(StorageError::Guard("HSK-403-SILENT-EDIT"))
    ));
    let remaining = storage
        .list_workspaces()
        .await
        .expect("list after rejected writes");
    assert_eq!(remaining.len(), 2);
    assert!(remaining
        .iter()
        .any(|workspace| workspace.id == existing.id && workspace.name == "Existing"));
    assert!(remaining
        .iter()
        .any(|workspace| workspace.id == ai_authored.id && workspace.name == "AI-authored"));
    assert!(!remaining
        .iter()
        .any(|workspace| workspace.name == "Blocked"));

    storage.shutdown().await.expect("close store");
}
