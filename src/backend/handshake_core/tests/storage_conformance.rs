use handshake_core::storage::tests::{
    postgres_backend_from_env, run_loom_storage_conformance, run_loom_traversal_performance_probe,
    run_storage_conformance,
};
use handshake_core::storage::StorageError;

#[tokio::test]
async fn postgres_storage_conformance() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_storage_conformance(db)
        .await
        .expect("postgres storage conformance");
}

#[tokio::test]
async fn postgres_loom_storage_conformance() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_loom_storage_conformance(db)
        .await
        .expect("postgres loom storage conformance");
}

#[tokio::test]
async fn postgres_loom_traversal_performance_target() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_loom_traversal_performance_probe(db)
        .await
        .expect("postgres loom traversal performance");
}
