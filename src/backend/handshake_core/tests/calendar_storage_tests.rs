use handshake_core::storage::tests::{postgres_backend_from_env, run_calendar_storage_conformance};
use handshake_core::storage::StorageError;

#[tokio::test]
async fn postgres_calendar_storage_conformance() {
    let db = match postgres_backend_from_env().await {
        Ok(db) => db,
        Err(err) => panic!("failed to init postgres backend: {err:?}"),
    };

    run_calendar_storage_conformance(db)
        .await
        .expect("postgres calendar storage conformance");
}
