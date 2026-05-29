use handshake_core::storage::tests::{postgres_backend_from_env, run_calendar_storage_conformance};
use handshake_core::storage::StorageError;

#[tokio::test]
async fn postgres_calendar_storage_conformance() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(StorageError::Validation(msg)) if msg.contains("POSTGRES_TEST_URL not set") => {
            eprintln!("Skipping postgres calendar storage conformance: {msg}");
            return;
        }
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_calendar_storage_conformance(db)
        .await
        .expect("postgres calendar storage conformance");
}
