use std::sync::Arc;

use handshake_core::storage::surreal::{
    bootstrap_schema, SurrealDataContext, SurrealStorage, SurrealStorageConfig,
    SurrealStorageError, DEFAULT_DATABASE, DEFAULT_NAMESPACE, DEFAULT_STORE_DIRECTORY,
};
use handshake_core::storage::{NewWorkspace, WriteContext};
use static_assertions::assert_not_impl_any;
use tokio::sync::{oneshot, Notify};
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn persistent_store_survives_shutdown_and_reopen() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let config =
        SurrealStorageConfig::for_data_dir(temp.path()).expect("resolve absolute store path");
    assert_eq!(
        config.path(),
        temp.path().join(DEFAULT_STORE_DIRECTORY).as_path()
    );
    assert_eq!(config.namespace(), DEFAULT_NAMESPACE);
    assert_eq!(config.database(), DEFAULT_DATABASE);

    let storage = SurrealStorage::open(config.clone())
        .await
        .expect("open persistent embedded store");
    bootstrap_schema(&storage)
        .await
        .expect("bootstrap schema before persistence write");
    let write_context = WriteContext::system(Some("surreal-lifecycle-test".to_owned()));
    let created = storage
        .create_workspace(
            &write_context,
            NewWorkspace {
                name: "persisted-across-restart".to_owned(),
            },
        )
        .await
        .expect("persist typed workspace record");

    storage.shutdown().await.expect("close embedded store");
    storage
        .shutdown()
        .await
        .expect("repeat close is idempotent");
    assert!(storage.is_closed().await);
    drop(storage);

    let reopened = SurrealStorage::open(config)
        .await
        .expect("reopen persistent embedded store");
    bootstrap_schema(&reopened)
        .await
        .expect("reverify schema after reopen");
    let record = reopened
        .get_workspace(&created.id)
        .await
        .expect("read persisted workspace record")
        .expect("persisted workspace exists after reopen");
    assert_eq!(record.id, created.id);
    assert_eq!(record.name, "persisted-across-restart");
    assert_eq!(record.created_at, created.created_at);
    assert_eq!(record.updated_at, created.updated_at);
    reopened.shutdown().await.expect("close reopened store");
}

#[tokio::test]
async fn cloned_wrappers_observe_one_shared_shutdown_state() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(temp.path().join("shared-shutdown"))
            .expect("resolve absolute store path"),
    )
    .await
    .expect("open embedded store");
    let clone = storage.clone();

    clone.shutdown().await.expect("close through clone");

    assert!(storage.is_closed().await);
    assert!(matches!(
        storage
            .with_data_operation(|_| Box::pin(async { Ok(()) }))
            .await,
        Err(SurrealStorageError::Closed)
    ));
    clone
        .shutdown()
        .await
        .expect("repeat clone close is idempotent");
}

#[tokio::test]
async fn shutdown_drains_operations_and_rejects_late_operations() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let config = SurrealStorageConfig::for_data_dir(temp.path().join("drain-gate"))
        .expect("resolve absolute store path");
    let storage = SurrealStorage::open(config.clone())
        .await
        .expect("open embedded store");

    let entered = Arc::new(Notify::new());
    let (release_tx, release_rx) = oneshot::channel();
    let active_storage = storage.clone();
    let active_entered = entered.clone();
    let active = tokio::spawn(async move {
        active_storage
            .with_data_operation(|_| {
                Box::pin(async move {
                    active_entered.notify_one();
                    release_rx.await.expect("release active operation");
                    Ok(())
                })
            })
            .await
    });
    entered.notified().await;

    let shutdown_storage = storage.clone();
    let shutdown = tokio::spawn(async move { shutdown_storage.shutdown().await });
    while storage.is_accepting_operations() {
        tokio::task::yield_now().await;
    }

    let late_storage = storage.clone();
    let late = tokio::spawn(async move {
        late_storage
            .with_data_operation(|_| Box::pin(async { Ok(()) }))
            .await
    });
    assert!(
        !shutdown.is_finished(),
        "shutdown must wait for active lease"
    );
    release_tx.send(()).expect("release active operation");

    active
        .await
        .expect("join active operation")
        .expect("active operation succeeds");
    shutdown
        .await
        .expect("join shutdown")
        .expect("shutdown succeeds");
    assert!(matches!(
        late.await.expect("join late operation"),
        Err(SurrealStorageError::Closed)
    ));

    drop(storage);
    let reopened = SurrealStorage::open(config)
        .await
        .expect("reopen after all clones close");
    reopened.shutdown().await.expect("close reopened store");
}

#[test]
fn data_operation_facade_has_no_clone_or_deref_escape() {
    assert_not_impl_any!(SurrealDataContext<'static>:
        Clone,
        std::ops::Deref,
        AsRef<surrealdb::Surreal<surrealdb::engine::local::Db>>,
        std::borrow::Borrow<surrealdb::Surreal<surrealdb::engine::local::Db>>
    );
}

#[test]
fn empty_data_root_is_rejected() {
    assert!(matches!(
        SurrealStorageConfig::for_data_dir(""),
        Err(SurrealStorageError::EmptyDataDirectory)
    ));

    let config = SurrealStorageConfig::for_data_dir("valid").expect("configure valid root");
    assert!(matches!(
        config.with_shutdown_wait_timeout(Duration::ZERO),
        Err(SurrealStorageError::InvalidShutdownWaitTimeout)
    ));
}

#[tokio::test]
async fn equivalent_store_paths_canonicalize_to_one_identity() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let data_root = temp.path().join("data-root");
    let alias_component = data_root.join("alias");
    std::fs::create_dir_all(&alias_component).expect("create path alias component");

    let direct = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(&data_root).expect("configure direct path"),
    )
    .await
    .expect("open direct path");
    let canonical_path = direct.config().path().to_path_buf();
    direct.shutdown().await.expect("close direct path");
    drop(direct);

    let equivalent_root = alias_component.join("..");
    let equivalent = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(equivalent_root).expect("configure equivalent path"),
    )
    .await
    .expect("open equivalent path");
    assert_eq!(equivalent.config().path(), canonical_path);
    equivalent.shutdown().await.expect("close equivalent path");
}

#[tokio::test]
async fn aborted_shutdown_caller_does_not_wedge_shared_close() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let config = SurrealStorageConfig::for_data_dir(temp.path().join("cancel-safe-close"))
        .expect("configure store");
    let storage = SurrealStorage::open(config.clone())
        .await
        .expect("open store");

    let entered = Arc::new(Notify::new());
    let (release_tx, release_rx) = oneshot::channel();
    let lease_storage = storage.clone();
    let lease_entered = entered.clone();
    let lease = tokio::spawn(async move {
        lease_storage
            .with_data_operation(|_| {
                Box::pin(async move {
                    lease_entered.notify_one();
                    release_rx.await.expect("release lease");
                    Ok(())
                })
            })
            .await
    });
    entered.notified().await;

    let first_storage = storage.clone();
    let first_caller = tokio::spawn(async move { first_storage.shutdown().await });
    while storage.is_accepting_operations() {
        tokio::task::yield_now().await;
    }
    first_caller.abort();
    assert!(first_caller
        .await
        .expect_err("first caller is aborted")
        .is_cancelled());

    let retry_a_storage = storage.clone();
    let retry_a = tokio::spawn(async move { retry_a_storage.shutdown().await });
    let retry_b_storage = storage.clone();
    let retry_b = tokio::spawn(async move { retry_b_storage.shutdown().await });
    release_tx.send(()).expect("release lease");
    lease.await.expect("join lease").expect("lease succeeds");
    retry_a
        .await
        .expect("join first retry")
        .expect("first shared shutdown waiter succeeds");
    retry_b
        .await
        .expect("join second retry")
        .expect("second shared shutdown waiter succeeds");
    assert!(storage.is_closed().await);

    drop(storage);
    let reopened = SurrealStorage::open(config)
        .await
        .expect("reopen closed path");
    reopened.shutdown().await.expect("close reopened path");
}

#[tokio::test]
async fn reentrant_shutdown_is_bounded_and_direct_shutdown_still_works() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let storage = SurrealStorage::open(
        SurrealStorageConfig::for_data_dir(temp.path().join("reentrant-close"))
            .expect("configure store"),
    )
    .await
    .expect("open store");
    let nested_storage = storage.clone();

    let nested = timeout(
        Duration::from_secs(1),
        storage
            .with_data_operation(move |_| Box::pin(async move { nested_storage.shutdown().await })),
    )
    .await
    .expect("reentrant shutdown must not deadlock");
    assert!(matches!(
        nested,
        Err(SurrealStorageError::ReentrantShutdown)
    ));
    assert!(storage.is_accepting_operations());

    timeout(Duration::from_secs(30), storage.shutdown())
        .await
        .expect("direct shutdown is bounded")
        .expect("direct shutdown succeeds");
}

#[tokio::test]
async fn spawned_child_shutdown_is_bounded_then_close_finishes_after_lease_drains() {
    let temp = tempfile::tempdir().expect("create temporary data root");
    let reopen_config = SurrealStorageConfig::for_data_dir(temp.path().join("spawned-child-close"))
        .expect("configure store");
    let cyclic_caller_config = reopen_config
        .clone()
        .with_shutdown_wait_timeout(Duration::from_millis(100))
        .expect("configure bounded shutdown wait");
    let storage = SurrealStorage::open(cyclic_caller_config)
        .await
        .expect("open store");
    let captured = storage.clone();
    let observed = storage.clone();

    let child_result = timeout(
        Duration::from_secs(2),
        storage.with_data_operation(move |_| {
            Box::pin(async move {
                let result = tokio::spawn(async move { captured.shutdown().await })
                    .await
                    .expect("join spawned shutdown");
                assert!(
                    !observed.is_closed().await,
                    "engine must not report closed while this operation still holds its lease"
                );
                result
            })
        }),
    )
    .await
    .expect("spawned-child shutdown must be bounded");
    assert!(matches!(
        child_result,
        Err(SurrealStorageError::ShutdownStillInProgress { .. })
    ));

    timeout(Duration::from_secs(5), async {
        while !storage.is_closed().await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("background shutdown finishes after operation lease drains");
    drop(storage);
    let reopened = SurrealStorage::open(reopen_config)
        .await
        .expect("reopen drained store");
    timeout(Duration::from_secs(30), reopened.shutdown())
        .await
        .expect("reopened shutdown is bounded")
        .expect("close reopened store");
}
