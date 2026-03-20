use handshake_core::storage::tests::{
    postgres_backend_from_env, run_loom_storage_conformance, run_loom_traversal_performance_probe,
    run_storage_conformance, sqlite_backend,
};
use handshake_core::storage::StorageError;

#[tokio::test]
async fn sqlite_storage_conformance() {
    let db = sqlite_backend().await.expect("sqlite backend");
    run_storage_conformance(db)
        .await
        .expect("sqlite storage conformance");
}

#[tokio::test]
async fn postgres_storage_conformance() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            eprintln!("Skipping postgres storage conformance: {msg}");
            return;
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_storage_conformance(db)
        .await
        .expect("postgres storage conformance");
}

#[tokio::test]
async fn sqlite_loom_storage_conformance() {
    let db = sqlite_backend().await.expect("sqlite backend");
    run_loom_storage_conformance(db)
        .await
        .expect("sqlite loom storage conformance");
}

#[tokio::test]
async fn postgres_loom_storage_conformance() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            eprintln!("Skipping postgres loom storage conformance: {msg}");
            return;
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_loom_storage_conformance(db)
        .await
        .expect("postgres loom storage conformance");
}

#[tokio::test]
async fn sqlite_loom_traversal_performance_target() {
    let db = sqlite_backend().await.expect("sqlite backend");
    run_loom_traversal_performance_probe(db)
        .await
        .expect("sqlite loom traversal performance");
}

#[tokio::test]
async fn postgres_loom_traversal_performance_target() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            eprintln!("Skipping postgres loom traversal performance: {msg}");
            return;
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_loom_traversal_performance_probe(db)
        .await
        .expect("postgres loom traversal performance");
}
