use handshake_core::storage::tests::{
    postgres_backend_from_env, run_storage_conformance, sqlite_backend,
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
